//! Puente entre la app de Tauri y el motor de VRes.
//!
//! Guarda lo pedido (`Requested`), lo persiste en `session.json` y reenvía a
//! la interfaz los eventos del motor.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use vres_core::ipc::Notice;
use vres_core::model::display::DisplayInfo;
use vres_core::model::state::{SESSION_SCHEMA, SessionState};
use vres_core::planner::Requested;
use vres_core::status::EngineStatus;
use vres_core::store::{self, Loaded};
use vres_engine::{ApplyMode, Command, EngineConfig, EngineEvent, EngineHandle, EventSink};

/// Evento: cambió el estado de alguna capa.
pub const EVENT_STATUS: &str = "engine:status";
/// Evento: cambiaron las pantallas.
pub const EVENT_DISPLAYS: &str = "displays:changed";
/// Evento: lo pedido cambió desde fuera de la interfaz (bandeja, atajo).
pub const EVENT_REQUESTED: &str = "state:requested";
/// Evento: aviso para el usuario.
pub const EVENT_NOTICE: &str = "app:notice";

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Estado del motor, administrado por Tauri.
pub struct EngineState {
    handle: Mutex<Option<EngineHandle>>,
    requested: Mutex<Requested>,
    displays: Mutex<Vec<DisplayInfo>>,
    status: Mutex<EngineStatus>,
    notices: Mutex<Vec<Notice>>,
    session_path: Option<PathBuf>,
}

impl EngineState {
    /// Arranca el motor con la sesión guardada. Si el motor no puede
    /// arrancar, VRes sigue abierto y la interfaz lo informa.
    pub fn start(app: &AppHandle) -> Self {
        let state_dir = vres_engine::state_dir();
        let session_path = state_dir.as_ref().map(|dir| dir.join("session.json"));
        let mut notices = Vec::new();

        let session = session_path
            .as_ref()
            .and_then(|path| match store::read_json::<SessionState>(path) {
                Loaded::Ok(session) => Some(session),
                _ => None,
            })
            .unwrap_or_default();
        let mut requested = session.requested;
        requested.compare = false;
        requested.paused = false;
        // Si la sesión anterior no cerró bien, se arranca con el color apagado
        // para no entrar en un bucle de fallos.
        if !session.clean_exit && requested.color_enabled {
            requested.color_enabled = false;
            notices.push(Notice::SafeMode);
        }

        let sink_app = app.clone();
        let sink: EventSink = Arc::new(move |event| on_engine_event(&sink_app, event));
        let config = EngineConfig {
            restore_path: vres_engine::restore_path(),
            requested: requested.clone(),
        };

        let state = match vres_engine::spawn(config, sink) {
            Ok((handle, snapshot)) => {
                if snapshot.recovered {
                    notices.push(Notice::Recovered);
                }
                if !snapshot.hotkey_registered {
                    notices.push(Notice::HotkeyTaken);
                }
                Self {
                    handle: Mutex::new(Some(handle)),
                    requested: Mutex::new(requested),
                    displays: Mutex::new(snapshot.displays),
                    status: Mutex::new(snapshot.status),
                    notices: Mutex::new(notices),
                    session_path,
                }
            }
            Err(error) => {
                tracing::error!(%error, "el motor no pudo arrancar");
                notices.push(Notice::EngineUnavailable);
                Self {
                    handle: Mutex::new(None),
                    requested: Mutex::new(requested),
                    displays: Mutex::new(Vec::new()),
                    status: Mutex::new(EngineStatus::default()),
                    notices: Mutex::new(notices),
                    session_path,
                }
            }
        };
        let current = state.requested();
        state.persist(&current, false);
        state
    }

    /// `true` si el motor está corriendo.
    pub fn available(&self) -> bool {
        lock(&self.handle).is_some()
    }

    /// Lo que está pedido.
    pub fn requested(&self) -> Requested {
        lock(&self.requested).clone()
    }

    /// Pantallas conocidas.
    pub fn displays(&self) -> Vec<DisplayInfo> {
        lock(&self.displays).clone()
    }

    /// Estado de las capas.
    pub fn status(&self) -> EngineStatus {
        lock(&self.status).clone()
    }

    /// Avisos pendientes para la interfaz (se vacían al leerlos).
    pub fn take_notices(&self) -> Vec<Notice> {
        std::mem::take(&mut *lock(&self.notices))
    }

    /// Envía un pedido al motor.
    pub fn send(&self, command: Command) -> bool {
        lock(&self.handle)
            .as_ref()
            .is_some_and(|handle| handle.send(command))
    }

    /// Modifica lo pedido, se lo envía al motor y, si es un cambio
    /// confirmado, lo guarda.
    pub fn update(&self, mode: ApplyMode, change: impl FnOnce(&mut Requested)) -> Requested {
        let snapshot = {
            let mut requested = lock(&self.requested);
            change(&mut requested);
            requested.clone()
        };
        self.send(Command::Set(Box::new(snapshot.clone()), mode));
        if mode == ApplyMode::Commit {
            self.persist(&snapshot, false);
        }
        snapshot
    }

    fn persist(&self, requested: &Requested, clean_exit: bool) {
        let Some(path) = &self.session_path else {
            return;
        };
        let mut stored = requested.clone();
        stored.compare = false;
        stored.paused = false;
        let session = SessionState {
            schema_version: SESSION_SCHEMA,
            clean_exit,
            requested: stored,
        };
        if let Err(error) = store::write_json_atomic(path, &session) {
            tracing::warn!(%error, "no se pudo guardar la sesión");
        }
    }

    /// Restaura todo, apaga el motor y marca la salida como limpia.
    pub fn shutdown(&self) {
        if let Some(mut handle) = lock(&self.handle).take()
            && !handle.shutdown(Duration::from_secs(3))
        {
            tracing::warn!("el motor no terminó a tiempo; restauración de emergencia");
            vres_engine::emergency_restore();
        }
        let requested = self.requested();
        self.persist(&requested, true);
    }
}

fn on_engine_event(app: &AppHandle, event: EngineEvent) {
    let Some(state) = app.try_state::<EngineState>() else {
        return;
    };
    match event {
        EngineEvent::Status(status) => {
            *lock(&state.status) = status.clone();
            let _ = app.emit(EVENT_STATUS, status);
        }
        EngineEvent::Displays(displays) => {
            *lock(&state.displays) = displays.clone();
            let _ = app.emit(EVENT_DISPLAYS, displays);
        }
        EngineEvent::EmergencyReset => {
            tracing::info!("restablecimiento de emergencia");
            let requested = {
                let mut requested = lock(&state.requested);
                requested.color_enabled = false;
                requested.paused = false;
                requested.compare = false;
                requested.clone()
            };
            state.persist(&requested, false);
            crate::tray::set_paused(app, false);
            let _ = app.emit(EVENT_REQUESTED, requested);
            let _ = app.emit(EVENT_NOTICE, Notice::EmergencyReset);
        }
        EngineEvent::QuitRequested => {
            tracing::info!("cierre pedido desde otro proceso");
            app.exit(0);
        }
        EngineEvent::Notice(reason) => {
            tracing::info!(?reason, "aviso del motor");
        }
    }
}

/// Pausa o reanuda todo desde la bandeja o un atajo, avisándole a la interfaz.
pub fn toggle_paused(app: &AppHandle) {
    let Some(state) = app.try_state::<EngineState>() else {
        return;
    };
    let requested = state.update(ApplyMode::Commit, |requested| {
        requested.paused = !requested.paused;
    });
    crate::tray::set_paused(app, requested.paused);
    let _ = app.emit(EVENT_REQUESTED, requested);
}
