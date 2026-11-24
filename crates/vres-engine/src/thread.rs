//! El hilo del motor: una ventana oculta y su bucle de mensajes.
//!
//! Solo este hilo toca la Magnification API, registra los atajos y recibe los
//! avisos del sistema. Los demás hilos le hablan por un canal
//! ([`EngineHandle::send`]) y él les responde con eventos ([`EngineEvent`]).
//! Los cambios de resolución y de HDR, que pueden tardar segundos, corren en
//! un hilo auxiliar: mientras tanto, el motor sigue atendiendo el atajo de
//! emergencia y la cuenta regresiva.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use vres_core::color::matrix::ColorMatrix;
use vres_core::color::ramp::GammaRamp;
use vres_core::crosshair::{self, Crosshair};
use vres_core::ipc::ErrorCode;
use vres_core::keys::Hotkey;
use vres_core::model::display::DisplayInfo;
use vres_core::model::resolution::{
    DisplayMode, DisplayModes, ModeRequest, ModeResolved, PendingMode, ResolutionState, is_listed,
    presets,
};
use vres_core::model::vibrance::VibranceInfo;
use vres_core::planner::Requested;
use vres_core::profiles::{Observed, ProfileSettings, SessionPlan, match_game, resolve};
use vres_core::settings::{CrosshairSettings, DEFAULT_CONFIRM_SECONDS, SavedMode};
use vres_core::status::{EngineStatus, ReasonCode};
use vres_win::control::{self, ACK, RemoteCommand};
use vres_win::identify::IdentifyOverlay;
use vres_win::mag::Magnifier;
use vres_win::overlay::CrosshairOverlay;
use vres_win::process::{self, ProcessInfo};
use vres_win::sound::{self, Tone};
use vres_win::vibrance::DriverVibrance;
use vres_win::window::{self, ForegroundHook, HiddenWindow};
use vres_win::{displays, gamma, keyboard, modes};

use crate::core::{Core, Hardware};
use crate::resolution::{Feedback, JobResult, ModeJob, Outcome, ResolutionManager};

// Mensajes y constantes públicas de la API de Windows.
const WM_CLOSE: u32 = 0x0010;
const WM_QUERYENDSESSION: u32 = 0x0011;
const WM_ENDSESSION: u32 = 0x0016;
const WM_DISPLAYCHANGE: u32 = 0x007E;
const WM_TIMER: u32 = 0x0113;
const WM_POWERBROADCAST: u32 = 0x0218;
const WM_WTSSESSION_CHANGE: u32 = 0x02B1;
const WM_HOTKEY: u32 = 0x0312;
const WM_APP: u32 = 0x8000;
const WTS_SESSION_UNLOCK: usize = 0x8;
const PBT_APMRESUMEAUTOMATIC: usize = 0x12;
const MOD_ALT: u32 = 0x1;
const MOD_CONTROL: u32 = 0x2;
const MOD_SHIFT: u32 = 0x4;
const MOD_NOREPEAT: u32 = 0x4000;
const VK_R: u32 = 0x52;

/// Hay comandos en el canal.
const WM_WAKE: u32 = WM_APP + 1;
/// Cambió la ventana en primer plano.
const WM_FOREGROUND: u32 = WM_APP + 2;
/// Terminó un trabajo de resolución o HDR.
const WM_JOB_DONE: u32 = WM_APP + 3;

const TIMER_SETTLE: usize = 1;
const TIMER_HEALTH: usize = 2;
const TIMER_IDENTIFY: usize = 3;
const TIMER_FOREGROUND: usize = 4;
const TIMER_MODE: usize = 5;
/// Mientras hay un juego abierto: ¿sigue abierto?
const TIMER_GAME: usize = 6;
const HOTKEY_EMERGENCY: i32 = 1;
const HOTKEY_TOGGLE: i32 = 2;

const HEALTH_INTERVAL_MS: u32 = 1000;
const IDENTIFY_MS: u32 = 2500;
const MODE_TICK_MS: u32 = 250;
const GAME_POLL_MS: u32 = 3000;
/// Tope para esperar un cambio de modo en curso antes de restaurar.
const JOB_WAIT: Duration = Duration::from_secs(2);

/// Ajustes del motor que vienen de `settings.json`.
#[derive(Debug, Clone)]
pub struct EngineSettings {
    /// Atajo que alterna entre la resolución de Windows y la estirada.
    pub toggle_hotkey: Option<Hotkey>,
    /// Destino del atajo: siempre un modo que el usuario ya conservó.
    pub toggle_target: Option<SavedMode>,
    /// Tonos al alternar con el atajo.
    pub tones: bool,
    /// Segundos para conservar un modo nuevo antes de que vuelva solo.
    pub confirm_seconds: u32,
    /// El crosshair: si está prendido, dónde y cuál.
    pub crosshair: CrosshairSettings,
    /// Perfiles por juego.
    pub profiles: ProfileSettings,
}

impl Default for EngineSettings {
    fn default() -> Self {
        Self {
            toggle_hotkey: None,
            toggle_target: None,
            tones: true,
            confirm_seconds: DEFAULT_CONFIRM_SECONDS,
            crosshair: CrosshairSettings::default(),
            profiles: ProfileSettings::default(),
        }
    }
}

/// Configuración del motor.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// `restore.json`. `None` desactiva la recuperación tras un cierre inesperado.
    pub restore_path: Option<PathBuf>,
    /// `restore-modes.json` (resolución y HDR).
    pub modes_restore_path: Option<PathBuf>,
    /// Lo que se aplica al arrancar.
    pub requested: Requested,
    /// Atajos y preferencias.
    pub settings: EngineSettings,
}

/// Vista previa (mientras se arrastra un control) o cambio confirmado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyMode {
    Preview,
    Commit,
}

/// Respuesta a un pedido.
pub type Reply<T> = Sender<Result<T, ErrorCode>>;

/// Pedidos al motor.
#[derive(Debug)]
pub enum Command {
    /// Aplicar lo pedido. Si llegan varios juntos, solo cuenta el último.
    Set(Box<Requested>, ApplyMode),
    /// Mostrar el número de cada pantalla.
    Identify,
    /// Volver todo a la normalidad y apagar el color.
    EmergencyReset,
    /// Volver a leer las pantallas y reaplicar.
    Refresh,
    /// Probar un modo: queda pendiente de confirmación.
    ModeApply {
        display_id: String,
        request: ModeRequest,
        reply: Reply<PendingMode>,
    },
    /// Conservar el modo pendiente.
    ModeConfirm { token: String, reply: Reply<()> },
    /// Volver atrás el modo pendiente.
    ModeRevert { token: String, reply: Reply<()> },
    /// Volver a la resolución de Windows en una pantalla.
    ModeRestore {
        display_id: String,
        reply: Reply<()>,
    },
    /// Lo mismo que el atajo: alternar entre la de Windows y la estirada.
    ModeToggle,
    /// Prender o apagar HDR.
    HdrSet {
        display_id: String,
        enabled: bool,
        reply: Reply<()>,
    },
    /// Nuevos ajustes. Responde si el atajo de resolución se pudo registrar.
    Configure {
        settings: Box<EngineSettings>,
        reply: Sender<bool>,
    },
    /// Suspender el atajo de resolución (mientras se graba uno nuevo).
    SuspendHotkeys(bool),
    /// Mostrar este crosshair mientras se edita (sin guardar). `None` vuelve
    /// al guardado.
    CrosshairPreview(Option<Box<Crosshair>>),
    /// Restaurar todo y terminar el hilo.
    Shutdown(Sender<()>),
}

/// Avisos del motor.
#[derive(Debug, Clone)]
pub enum EngineEvent {
    /// Cambió el estado de alguna capa.
    Status(EngineStatus),
    /// Cambiaron las pantallas.
    Displays(Vec<DisplayInfo>),
    /// Cambiaron los modos, o hay un cambio pendiente.
    Resolution(ResolutionState),
    /// Terminó un cambio de resolución.
    ModeResolved(ModeResolved),
    /// Se restableció todo (atajo de emergencia u otro proceso): el color quedó apagado.
    EmergencyReset,
    /// Otro proceso pidió cerrar VRes (`vres.exe --shutdown`, el instalador).
    QuitRequested,
    /// Aviso para el usuario.
    Notice(ReasonCode),
    /// Detalle técnico para el log (en inglés).
    Log(String),
    /// Cambió el perfil en uso (`None`: el escritorio).
    Profile(Option<String>),
}

/// Estado inicial, al terminar de arrancar.
#[derive(Debug, Clone)]
pub struct EngineSnapshot {
    pub displays: Vec<DisplayInfo>,
    pub status: EngineStatus,
    pub resolution: ResolutionState,
    /// Se restauraron cambios de una sesión que no cerró bien.
    pub recovered: bool,
    /// Se pudo registrar el atajo de emergencia (`Ctrl + Alt + Shift + R`).
    pub hotkey_registered: bool,
    /// Se pudo registrar el atajo de resolución (o no hay ninguno configurado).
    pub toggle_hotkey_registered: bool,
}

/// Carpeta de estado de la máquina: `%LOCALAPPDATA%\io.github.gxnza48.vres\state`.
pub fn state_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|dir| {
        PathBuf::from(dir)
            .join(vres_core::APP_IDENTIFIER)
            .join("state")
    })
}

/// `restore.json`: los valores originales mientras VRes tiene cambios aplicados.
pub fn restore_path() -> Option<PathBuf> {
    state_dir().map(|dir| dir.join("restore.json"))
}

/// `restore-modes.json`: lo que hay que deshacer de resolución y HDR.
pub fn modes_restore_path() -> Option<PathBuf> {
    state_dir().map(|dir| dir.join("restore-modes.json"))
}

/// Si quedaron cambios de una sesión que no cerró bien, los deshace sin
/// arrancar la app (lo usan `vres.exe --reset` y `vres-cli reset`).
/// Devuelve `true` si había algo que restaurar.
pub fn recover_from_disk() -> bool {
    let found = displays::enumerate().unwrap_or_default();
    let mut recovered = false;
    if let Some(path) = restore_path() {
        let mut core = Core::new(
            WinHardware {
                magnifier: None,
                vibrance: DriverVibrance::load(),
            },
            Some(path),
        );
        recovered |= core.recover(&found);
    }
    let mut manager = ResolutionManager::new(modes_restore_path());
    for job in manager.recovery_jobs(&found) {
        let _ = execute_job(&job);
        recovered = true;
    }
    recovered
}

/// Función que recibe los eventos. Corre en el hilo del motor: no debe bloquear.
pub type EventSink = Arc<dyn Fn(EngineEvent) + Send + Sync>;

/// Manija del motor, para hablarle desde cualquier hilo.
#[derive(Debug)]
pub struct EngineHandle {
    tx: Sender<Command>,
    hwnd: isize,
    wake_pending: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl EngineHandle {
    /// Envía un pedido. Devuelve `false` si el motor ya terminó.
    pub fn send(&self, command: Command) -> bool {
        if self.tx.send(command).is_err() {
            return false;
        }
        // Como mucho un aviso pendiente: el motor vacía el canal entero.
        if !self.wake_pending.swap(true, Ordering::AcqRel) {
            window::post_message(self.hwnd, WM_WAKE, 0, 0);
        }
        true
    }

    /// Restaura todo y espera a que el motor termine (con tope `timeout`).
    pub fn shutdown(&mut self, timeout: Duration) -> bool {
        let (reply_tx, reply_rx) = mpsc::channel();
        if !self.send(Command::Shutdown(reply_tx)) {
            return false;
        }
        let done = reply_rx.recv_timeout(timeout).is_ok();
        if done && let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        done
    }
}

/// Rampas originales de las pantallas modificadas, para restaurar desde
/// cualquier hilo (por ejemplo, el hook de pánico).
static EMERGENCY_ORIGINALS: Mutex<Vec<(String, GammaRamp)>> = Mutex::new(Vec::new());
/// Trabajos que deshacen los cambios de resolución y HDR vigentes.
static EMERGENCY_MODE_JOBS: Mutex<Vec<ModeJob>> = Mutex::new(Vec::new());
/// Vibrance original de las pantallas que VRes modificó.
static EMERGENCY_VIBRANCE: Mutex<Vec<(DisplayInfo, i32)>> = Mutex::new(Vec::new());

/// Restauración de emergencia. Se puede llamar desde cualquier hilo. La matriz
/// de color no hace falta: Windows la quita sola al terminar el proceso.
pub fn emergency_restore() {
    for _ in 0..20 {
        if let Ok(originals) = EMERGENCY_ORIGINALS.try_lock() {
            for (gdi_name, ramp) in originals.iter() {
                let _ = gamma::write_ramp(gdi_name, ramp);
            }
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    if let Ok(originals) = EMERGENCY_VIBRANCE.try_lock()
        && !originals.is_empty()
    {
        let mut driver = DriverVibrance::load();
        for (display, raw) in originals.iter() {
            let _ = driver.set(display, *raw);
        }
    }
    for _ in 0..20 {
        if let Ok(jobs) = EMERGENCY_MODE_JOBS.try_lock() {
            for job in jobs.iter() {
                let _ = execute_job(job);
            }
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

struct WinHardware {
    magnifier: Option<Magnifier>,
    vibrance: DriverVibrance,
}

impl Hardware for WinHardware {
    fn set_matrix(&mut self, matrix: &ColorMatrix) -> Result<(), String> {
        let magnifier = self
            .magnifier
            .as_ref()
            .ok_or_else(|| "magnification api unavailable".to_owned())?;
        magnifier
            .set_color_effect(matrix)
            .map_err(|error| error.to_string())
    }

    fn matrix(&mut self) -> Result<ColorMatrix, String> {
        let magnifier = self
            .magnifier
            .as_ref()
            .ok_or_else(|| "magnification api unavailable".to_owned())?;
        magnifier.color_effect().map_err(|error| error.to_string())
    }

    fn read_ramp(&mut self, gdi_name: &str) -> Result<GammaRamp, String> {
        gamma::read_ramp(gdi_name).map_err(|error| error.to_string())
    }

    fn write_ramp(&mut self, gdi_name: &str, ramp: &GammaRamp) -> Result<bool, String> {
        gamma::write_ramp(gdi_name, ramp).map_err(|error| error.to_string())
    }

    fn vibrance(&mut self, display: &DisplayInfo) -> Option<VibranceInfo> {
        self.vibrance.info(display)
    }

    fn set_vibrance(&mut self, display: &DisplayInfo, raw: i32) -> Result<(), String> {
        self.vibrance
            .set(display, raw)
            .map_err(|error| error.to_string())
    }

    fn forget_displays(&mut self) {
        self.vibrance.forget_displays();
    }
}

/// Ejecuta un trabajo de resolución o HDR. Corre en un hilo auxiliar (o en el
/// del motor al restaurar).
fn execute_job(job: &ModeJob) -> JobResult {
    let result = match job {
        ModeJob::Apply { gdi_name, request } => modes::apply_scaled(gdi_name, request).map(|got| {
            modes::scaling_ignored(request.scaling, got).then_some(ReasonCode::ScalingIgnored)
        }),
        ModeJob::Revert { gdi_name } => {
            let unchanged = matches!(
                (modes::current_mode(gdi_name), modes::saved_mode(gdi_name)),
                (Ok(current), Ok(saved)) if current == saved
            );
            if unchanged {
                Ok(None)
            } else {
                modes::revert_to_saved(gdi_name).map(|()| None)
            }
        }
        ModeJob::SetHdr { gdi_name, enabled } => modes::set_hdr(gdi_name, *enabled).map(|()| None),
    };
    match result {
        Ok(reason) => JobResult::Done { reason },
        Err(error) => JobResult::Failed {
            detail: error.to_string(),
        },
    }
}

/// Modos y estado de una pantalla, leídos de Windows.
fn display_modes(display: &DisplayInfo, changed: bool) -> DisplayModes {
    let gdi_name = &display.gdi_name;
    let list = modes::list_modes(gdi_name);
    let current = modes::current_mode(gdi_name).unwrap_or(DisplayMode::new(
        display.bounds.width,
        display.bounds.height,
        display.refresh_hz.round() as u32,
    ));
    let saved = modes::saved_mode(gdi_name).unwrap_or(current);
    let native = modes::native_mode(gdi_name).or_else(|| list.first().copied());
    let base = native.unwrap_or(saved);
    // Los presets prefieren la frecuencia de siempre del usuario.
    let preset_base = DisplayMode::new(base.width, base.height, saved.refresh_hz);
    DisplayModes {
        display_id: display.id.clone(),
        current,
        saved,
        native,
        scaling: modes::scaling(gdi_name).unwrap_or_default(),
        presets: presets(&preset_base, &list),
        modes: list,
        changed,
    }
}

/// A quién responderle cuando termine el trabajo en curso.
enum JobReply {
    Pending(Reply<PendingMode>),
    Unit(Reply<()>),
}

struct Shell {
    core: Core<WinHardware>,
    modes: ResolutionManager,
    settings: EngineSettings,
    sink: EventSink,
    rx: Receiver<Command>,
    wake_pending: Arc<AtomicBool>,
    remote_message: u32,
    hwnd: isize,
    identify: Option<IdentifyOverlay>,
    displays_dirty: bool,
    health_armed: bool,
    mode_timer_armed: bool,
    hotkeys_suspended: bool,
    job_tx: Sender<JobResult>,
    job_rx: Receiver<JobResult>,
    job_reply: Option<JobReply>,
    resolution_rev: u64,
    overlay: Option<CrosshairOverlay>,
    crosshair_preview: Option<Crosshair>,
    /// Lo que pidió el usuario para el escritorio (sin los perfiles).
    desktop: Requested,
    /// El juego abierto que tiene la sesión (el último que estuvo al frente).
    game: Option<(String, ProcessInfo)>,
    /// Lo que aplican ahora los perfiles.
    session: SessionPlan,
    /// Resolución que aplicó un perfil (para revertirla al cerrar el juego).
    profile_resolution: Option<SavedMode>,
    game_timer_armed: bool,
    shutdown_reply: Option<Sender<()>>,
}

impl Shell {
    fn on_message(&mut self, msg: u32, wparam: usize) -> Option<isize> {
        match msg {
            WM_WAKE => {
                self.drain();
                Some(0)
            }
            WM_JOB_DONE => {
                self.on_job_done();
                Some(0)
            }
            WM_HOTKEY => {
                if wparam == HOTKEY_EMERGENCY as usize {
                    self.emergency_reset();
                } else if wparam == HOTKEY_TOGGLE as usize {
                    self.toggle();
                }
                Some(0)
            }
            WM_TIMER => {
                self.on_timer(wparam);
                Some(0)
            }
            WM_DISPLAYCHANGE => {
                self.displays_dirty = true;
                window::set_timer(self.hwnd, TIMER_SETTLE, 500);
                None
            }
            WM_WTSSESSION_CHANGE => {
                if wparam == WTS_SESSION_UNLOCK {
                    window::set_timer(self.hwnd, TIMER_SETTLE, 300);
                }
                None
            }
            WM_POWERBROADCAST => {
                if wparam == PBT_APMRESUMEAUTOMATIC {
                    self.displays_dirty = true;
                    window::set_timer(self.hwnd, TIMER_SETTLE, 1000);
                }
                None
            }
            WM_FOREGROUND => {
                // El juego que pasa al frente puede taparlo: se vuelve a subir.
                if let Some(overlay) = &self.overlay {
                    overlay.raise();
                }
                // Un juego a pantalla completa puede pisar la rampa (se
                // verifica) y puede tener un perfil (se mira cuál es).
                if self.core.is_active() || self.settings.profiles.auto_switch {
                    window::set_timer(self.hwnd, TIMER_FOREGROUND, 150);
                }
                Some(0)
            }
            WM_QUERYENDSESSION => Some(1),
            WM_ENDSESSION => {
                if wparam != 0 {
                    self.restore_modes();
                    self.core.restore_all();
                    self.after_change(true);
                }
                Some(0)
            }
            message if message == self.remote_message => Some(self.on_remote(wparam)),
            _ => None,
        }
    }

    fn on_remote(&mut self, wparam: usize) -> isize {
        match RemoteCommand::from_wparam(wparam) {
            Some(RemoteCommand::EmergencyReset) => self.emergency_reset(),
            // La app decide cómo salir; el motor restaura en `Shutdown`.
            Some(RemoteCommand::Shutdown) => (self.sink)(EngineEvent::QuitRequested),
            Some(RemoteCommand::Ping) => {}
            None => return 0,
        }
        ACK
    }

    fn on_timer(&mut self, id: usize) {
        match id {
            TIMER_SETTLE => {
                window::kill_timer(self.hwnd, TIMER_SETTLE);
                if std::mem::take(&mut self.displays_dirty) {
                    self.refresh_displays();
                    self.emit_resolution();
                    self.apply_crosshair();
                }
                // Windows resetea las rampas en los cambios de pantalla: se reescribe todo.
                let changed = self.core.reconcile(true);
                self.after_change(changed);
            }
            TIMER_HEALTH => {
                let changed = self.core.health_check();
                self.after_change(changed);
            }
            TIMER_FOREGROUND => {
                window::kill_timer(self.hwnd, TIMER_FOREGROUND);
                if self.core.is_active() {
                    let changed = self.core.health_check();
                    self.after_change(changed);
                }
                self.observe_games();
            }
            TIMER_GAME => {
                let ended = self
                    .game
                    .as_ref()
                    .is_some_and(|(_, process)| !process::is_running(process));
                if ended {
                    self.game = None;
                }
                self.observe_games();
            }
            TIMER_IDENTIFY => {
                window::kill_timer(self.hwnd, TIMER_IDENTIFY);
                self.identify = None;
            }
            TIMER_MODE => {
                if let Some(job) = self.modes.tick(Instant::now()) {
                    self.run_job(job);
                }
                if self.modes.pending(Instant::now()).is_none() {
                    self.stop_mode_timer();
                }
            }
            _ => {}
        }
    }

    fn drain(&mut self) {
        self.wake_pending.store(false, Ordering::Release);
        let mut latest: Option<Requested> = None;
        let (mut identify, mut emergency, mut refresh) = (false, false, false);
        let mut shutdown = None;
        let mut ordered = Vec::new();
        while let Ok(command) = self.rx.try_recv() {
            match command {
                Command::Set(requested, _) => latest = Some(*requested),
                Command::Identify => identify = true,
                Command::EmergencyReset => emergency = true,
                Command::Refresh => refresh = true,
                Command::Shutdown(reply) => shutdown = Some(reply),
                other => ordered.push(other),
            }
        }

        if let Some(requested) = latest {
            self.desktop = requested;
            self.apply_effective();
        }
        if refresh {
            self.displays_dirty = true;
            window::set_timer(self.hwnd, TIMER_SETTLE, 10);
        }
        if identify {
            self.identify();
        }
        for command in ordered {
            self.on_command(command);
        }
        if emergency {
            self.emergency_reset();
        }
        if let Some(reply) = shutdown {
            self.restore_modes();
            self.core.restore_all();
            self.after_change(true);
            self.identify = None;
            self.shutdown_reply = Some(reply);
            window::post_message(self.hwnd, WM_CLOSE, 0, 0);
        }
    }

    fn display(&self, display_id: &str) -> Option<DisplayInfo> {
        self.core
            .displays()
            .iter()
            .find(|display| display.id == display_id)
            .cloned()
    }

    fn on_command(&mut self, command: Command) {
        match command {
            Command::ModeApply {
                display_id,
                request,
                reply,
            } => {
                let Some(display) = self.display(&display_id) else {
                    let _ = reply.send(Err(ErrorCode::DisplayGone));
                    return;
                };
                let listed = is_listed(&modes::list_modes(&display.gdi_name), &request);
                let seconds = self.settings.confirm_seconds;
                match self.modes.start_try(&display, request, listed, seconds) {
                    Ok(job) => {
                        self.job_reply = Some(JobReply::Pending(reply));
                        self.run_job(job);
                    }
                    Err(code) => {
                        let _ = reply.send(Err(code));
                    }
                }
            }
            Command::ModeConfirm { token, reply } => match self.modes.confirm(&token) {
                Ok(resolved) => {
                    let _ = reply.send(Ok(()));
                    self.stop_mode_timer();
                    (self.sink)(EngineEvent::ModeResolved(resolved));
                    self.emit_resolution();
                }
                Err(code) => {
                    let _ = reply.send(Err(code));
                }
            },
            Command::ModeRevert { token, reply } => match self.modes.start_revert(&token) {
                Ok(job) => {
                    self.job_reply = Some(JobReply::Unit(reply));
                    self.run_job(job);
                }
                Err(code) => {
                    let _ = reply.send(Err(code));
                }
            },
            Command::ModeRestore { display_id, reply } => {
                let Some(display) = self.display(&display_id) else {
                    let _ = reply.send(Err(ErrorCode::DisplayGone));
                    return;
                };
                match self.modes.start_restore_display(&display, false) {
                    Ok(Some(job)) => {
                        self.job_reply = Some(JobReply::Unit(reply));
                        self.run_job(job);
                    }
                    Ok(None) => {
                        let _ = reply.send(Ok(()));
                    }
                    Err(code) => {
                        let _ = reply.send(Err(code));
                    }
                }
            }
            Command::ModeToggle => self.toggle(),
            Command::HdrSet {
                display_id,
                enabled,
                reply,
            } => {
                let Some(display) = self.display(&display_id) else {
                    let _ = reply.send(Err(ErrorCode::DisplayGone));
                    return;
                };
                match self.modes.start_hdr(&display, enabled) {
                    Ok(Some(job)) => {
                        self.job_reply = Some(JobReply::Unit(reply));
                        self.run_job(job);
                    }
                    Ok(None) => {
                        let _ = reply.send(Ok(()));
                    }
                    Err(code) => {
                        let _ = reply.send(Err(code));
                    }
                }
            }
            Command::Configure { settings, reply } => {
                self.settings = *settings;
                self.crosshair_preview = None;
                let registered = self.register_toggle_hotkey();
                self.apply_crosshair();
                self.observe_games();
                let _ = reply.send(registered);
            }
            Command::CrosshairPreview(crosshair) => {
                self.crosshair_preview = crosshair.map(|crosshair| *crosshair);
                self.apply_crosshair();
            }
            Command::SuspendHotkeys(suspended) => {
                self.hotkeys_suspended = suspended;
                self.register_toggle_hotkey();
            }
            Command::Set(..)
            | Command::Identify
            | Command::EmergencyReset
            | Command::Refresh
            | Command::Shutdown(_) => {}
        }
    }

    /// Dibuja (o esconde) el crosshair según los ajustes y la vista previa.
    fn apply_crosshair(&mut self) {
        let config = self.settings.crosshair.clone();
        // Un perfil de juego puede prenderlo o apagarlo mientras está al frente.
        let enabled = self.session.crosshair.unwrap_or(config.enabled);
        if !enabled {
            if let Some(overlay) = &mut self.overlay {
                overlay.hide();
            }
            return;
        }
        let crosshair = self
            .crosshair_preview
            .clone()
            .unwrap_or(config.current)
            .clamped();
        let display = config
            .display_id
            .as_deref()
            .and_then(|id| self.display(id))
            .or_else(|| self.core.displays().iter().find(|d| d.primary).cloned())
            .or_else(|| self.core.displays().first().cloned());
        let Some(display) = display else {
            return;
        };
        let rendered = match crosshair::render(&crosshair) {
            Ok(rendered) => rendered,
            Err(error) => {
                (self.sink)(EngineEvent::Log(format!("crosshair not drawn: {error}")));
                if let Some(overlay) = &mut self.overlay {
                    overlay.hide();
                }
                return;
            }
        };
        if self.overlay.is_none() {
            self.overlay = CrosshairOverlay::create().ok();
        }
        let Some(overlay) = &mut self.overlay else {
            return;
        };
        let bounds = display.bounds;
        let center_x = bounds.x + (bounds.width / 2) as i32;
        let center_y = bounds.y + (bounds.height / 2) as i32;
        let x = center_x + crosshair.offset_x - rendered.center as i32;
        let y = center_y + crosshair.offset_y - rendered.center as i32;
        overlay.set_hidden_from_capture(config.hide_from_capture);
        if let Err(error) = overlay.show(&rendered.to_bgra_premultiplied(), rendered.size, x, y) {
            (self.sink)(EngineEvent::Log(format!(
                "crosshair overlay failed: {error}"
            )));
        }
    }

    /// (Re)registra el atajo de resolución. Devuelve `false` si otro programa
    /// ya lo usa o la tecla no se puede registrar.
    fn register_toggle_hotkey(&mut self) -> bool {
        window::unregister_hotkey(self.hwnd, HOTKEY_TOGGLE);
        if self.hotkeys_suspended {
            return true;
        }
        let Some(hotkey) = &self.settings.toggle_hotkey else {
            return true;
        };
        match keyboard::hotkey_registration(hotkey) {
            Some((modifiers, vk)) => {
                window::register_hotkey(self.hwnd, HOTKEY_TOGGLE, modifiers, vk).is_ok()
            }
            None => false,
        }
    }

    fn toggle(&mut self) {
        let Some(target) = self.settings.toggle_target.clone() else {
            return;
        };
        let Some(display) = self.display(&target.display_id) else {
            return;
        };
        let listed = is_listed(&modes::list_modes(&display.gdi_name), &target.mode);
        match self.modes.start_toggle(&target, true, &display, listed) {
            Ok(job) => self.run_job(job),
            Err(code) => (self.sink)(EngineEvent::Log(format!("toggle refused: {code:?}"))),
        }
    }

    fn run_job(&mut self, job: ModeJob) {
        self.update_emergency_jobs();
        let tx = self.job_tx.clone();
        let hwnd = self.hwnd;
        let worker_job = job.clone();
        let spawned = std::thread::Builder::new()
            .name("vres-modes".into())
            .spawn(move || {
                let result = execute_job(&worker_job);
                let _ = tx.send(result);
                window::post_message(hwnd, WM_JOB_DONE, 0, 0);
            });
        if spawned.is_err() {
            // Sin hilo auxiliar, se hace acá mismo.
            let result = execute_job(&job);
            let outcome = self.modes.finish(result, Instant::now());
            self.apply_outcome(outcome);
        }
    }

    fn on_job_done(&mut self) {
        while let Ok(result) = self.job_rx.try_recv() {
            if let JobResult::Failed { detail } = &result {
                (self.sink)(EngineEvent::Log(format!(
                    "display mode job failed: {detail}"
                )));
            }
            let outcome = self.modes.finish(result, Instant::now());
            self.apply_outcome(outcome);
        }
    }

    fn apply_outcome(&mut self, outcome: Outcome) {
        if let Some(reply) = outcome.reply {
            match self.job_reply.take() {
                Some(JobReply::Pending(tx)) => {
                    let _ = tx.send(reply.and_then(|pending| pending.ok_or(ErrorCode::ModeFailed)));
                }
                Some(JobReply::Unit(tx)) => {
                    let _ = tx.send(reply.map(|_| ()));
                }
                None => {}
            }
        }
        if outcome.pending.is_some() {
            self.start_mode_timer();
        }
        if let Some(resolved) = outcome.resolved {
            if self.modes.pending(Instant::now()).is_none() {
                self.stop_mode_timer();
            }
            (self.sink)(EngineEvent::ModeResolved(resolved));
        }
        if self.settings.tones {
            match outcome.feedback {
                Some(Feedback::ToneUp) => sound::play(Tone::Up),
                Some(Feedback::ToneDown) => sound::play(Tone::Down),
                Some(Feedback::None) | None => {}
            }
        }
        self.update_emergency_jobs();
        if outcome.changed {
            // El HDR cambia la información de la pantalla; el modo, sus medidas.
            self.refresh_displays();
            self.emit_resolution();
        }
    }

    fn start_mode_timer(&mut self) {
        if !self.mode_timer_armed {
            window::set_timer(self.hwnd, TIMER_MODE, MODE_TICK_MS);
            self.mode_timer_armed = true;
        }
    }

    fn stop_mode_timer(&mut self) {
        if self.mode_timer_armed {
            window::kill_timer(self.hwnd, TIMER_MODE);
            self.mode_timer_armed = false;
        }
    }

    /// Deshace los cambios de resolución y HDR, en este mismo hilo.
    fn restore_modes(&mut self) {
        if self.modes.busy()
            && let Ok(result) = self.job_rx.recv_timeout(JOB_WAIT)
        {
            let _ = self.modes.finish(result, Instant::now());
        }
        let had_changes = self.modes.has_changes();
        let jobs = self.modes.restore_jobs(self.core.displays());
        for job in &jobs {
            if let JobResult::Failed { detail } = execute_job(job) {
                (self.sink)(EngineEvent::Log(format!("restore job failed: {detail}")));
            }
        }
        if had_changes || !jobs.is_empty() {
            self.modes.restored();
        }
        self.job_reply = None;
        self.stop_mode_timer();
        self.update_emergency_jobs();
    }

    fn update_emergency_jobs(&self) {
        if let Ok(mut jobs) = EMERGENCY_MODE_JOBS.lock() {
            *jobs = self.modes.emergency_jobs(self.core.displays());
        }
    }

    fn resolution_state(&mut self) -> ResolutionState {
        self.resolution_rev += 1;
        let displays = self
            .core
            .displays()
            .iter()
            .map(|display| display_modes(display, self.modes.changed(&display.id).is_some()))
            .collect();
        ResolutionState {
            rev: self.resolution_rev,
            displays,
            pending: self.modes.pending(Instant::now()),
        }
    }

    fn emit_resolution(&mut self) {
        let state = self.resolution_state();
        (self.sink)(EngineEvent::Resolution(state));
    }

    fn refresh_displays(&mut self) {
        let Ok(found) = displays::enumerate() else {
            return;
        };
        if found.as_slice() != self.core.displays() {
            self.core.set_displays(found.clone());
            if let Some(resolved) = self.modes.displays_changed(&found) {
                self.stop_mode_timer();
                (self.sink)(EngineEvent::ModeResolved(resolved));
            }
            (self.sink)(EngineEvent::Displays(found));
        }
    }

    /// Lo pedido para el escritorio, con el look del perfil si hay un juego al frente.
    fn effective_requested(&self) -> Requested {
        let mut requested = self.desktop.clone();
        if let Some(look) = &self.session.look {
            requested.look = look.clone();
        }
        requested
    }

    fn apply_effective(&mut self) {
        let effective = self.effective_requested();
        self.core.set_requested(effective);
        let changed = self.core.reconcile(false);
        self.after_change(changed);
    }

    /// Mira qué juego está al frente y aplica (o quita) su perfil.
    fn observe_games(&mut self) {
        let profiles = self.settings.profiles.clone();
        if profiles.auto_switch {
            let foreground = process::foreground_process().and_then(|process| {
                match_game(&process.exe, &profiles.custom_games).map(|id| (id, process))
            });
            if let Some((id, process)) = &foreground
                && self.game.as_ref().map(|(_, current)| current) != Some(process)
            {
                self.game = Some((id.clone(), process.clone()));
            }
            let observed = Observed {
                foreground: foreground.map(|(id, _)| id),
                running: self.game.as_ref().map(|(id, _)| id.clone()),
            };
            self.apply_session(resolve(&profiles, &observed));
        } else {
            self.game = None;
            self.apply_session(SessionPlan::default());
        }
        // Mientras haya un juego abierto, cada 3 s se mira si sigue abierto.
        let want_timer = self.game.is_some();
        if want_timer && !self.game_timer_armed {
            window::set_timer(self.hwnd, TIMER_GAME, GAME_POLL_MS);
        } else if !want_timer && self.game_timer_armed {
            window::kill_timer(self.hwnd, TIMER_GAME);
        }
        self.game_timer_armed = want_timer;
    }

    fn apply_session(&mut self, plan: SessionPlan) {
        let active_changed = plan.active != self.session.active;
        let look_changed = plan.look != self.session.look;
        let crosshair_changed = plan.crosshair != self.session.crosshair;
        let wanted_resolution = plan.resolution.clone();
        self.session = plan;
        if active_changed {
            (self.sink)(EngineEvent::Profile(self.session.active.clone()));
        }
        if look_changed {
            self.apply_effective();
        }
        if crosshair_changed {
            self.apply_crosshair();
        }
        if wanted_resolution != self.profile_resolution {
            self.apply_profile_resolution(wanted_resolution);
        }
    }

    /// Lleva la resolución a la que pide el perfil (o la de Windows al cerrar
    /// el juego). Si hay otro cambio en curso, se reintenta en el próximo aviso.
    fn apply_profile_resolution(&mut self, wanted: Option<SavedMode>) {
        if self.modes.busy() || self.modes.pending(Instant::now()).is_some() {
            return;
        }
        if let Some(previous) = self.profile_resolution.clone()
            && wanted.as_ref() != Some(&previous)
        {
            if let Some(display) = self.display(&previous.display_id)
                && let Ok(Some(job)) = self.modes.start_restore_display(&display, false)
            {
                self.profile_resolution = None;
                self.run_job(job);
                return;
            }
            self.profile_resolution = None;
        }
        let Some(target) = wanted else {
            return;
        };
        let Some(display) = self.display(&target.display_id) else {
            return;
        };
        let listed = is_listed(&modes::list_modes(&display.gdi_name), &target.mode);
        match self.modes.start_ensure(&target, &display, listed) {
            Ok(Some(job)) => {
                self.profile_resolution = Some(target);
                self.run_job(job);
            }
            Ok(None) => self.profile_resolution = Some(target),
            Err(code) => (self.sink)(EngineEvent::Log(format!(
                "profile resolution refused: {code:?}"
            ))),
        }
    }

    fn emergency_reset(&mut self) {
        self.desktop.color_enabled = false;
        let requested = self.effective_requested();
        self.core.set_requested(requested);
        self.core.restore_all();
        let had_modes = self.modes.has_changes();
        self.restore_modes();
        self.after_change(true);
        if had_modes {
            self.refresh_displays();
            self.emit_resolution();
        }
        (self.sink)(EngineEvent::EmergencyReset);
    }

    fn identify(&mut self) {
        let entries: Vec<_> = self
            .core
            .displays()
            .iter()
            .enumerate()
            .map(|(index, display)| (display.bounds, index as u32 + 1))
            .collect();
        self.identify = None;
        if let Ok(overlay) = IdentifyOverlay::show(&entries) {
            self.identify = Some(overlay);
            window::set_timer(self.hwnd, TIMER_IDENTIFY, IDENTIFY_MS);
        }
    }

    fn after_change(&mut self, status_changed: bool) {
        if status_changed {
            (self.sink)(EngineEvent::Status(self.core.status().clone()));
        }
        if let Ok(mut originals) = EMERGENCY_ORIGINALS.lock() {
            *originals = self.core.originals_to_restore();
        }
        if let Ok(mut originals) = EMERGENCY_VIBRANCE.lock() {
            *originals = self.core.vibrance_to_restore();
        }
        let active = self.core.is_active();
        if active && !self.health_armed {
            window::set_timer(self.hwnd, TIMER_HEALTH, HEALTH_INTERVAL_MS);
        } else if !active && self.health_armed {
            window::kill_timer(self.hwnd, TIMER_HEALTH);
        }
        self.health_armed = active;
    }
}

/// Arranca el motor en su propio hilo. Devuelve la manija y el estado inicial.
pub fn spawn(
    config: EngineConfig,
    sink: EventSink,
) -> Result<(EngineHandle, EngineSnapshot), String> {
    let (tx, rx) = mpsc::channel::<Command>();
    let (ready_tx, ready_rx) = mpsc::channel::<Result<(isize, EngineSnapshot), String>>();
    let wake_pending = Arc::new(AtomicBool::new(false));
    let thread_wake = Arc::clone(&wake_pending);

    let thread = std::thread::Builder::new()
        .name("vres-engine".into())
        .spawn(move || run(config, sink, rx, thread_wake, ready_tx))
        .map_err(|error| error.to_string())?;

    match ready_rx.recv_timeout(Duration::from_secs(15)) {
        Ok(Ok((hwnd, snapshot))) => Ok((
            EngineHandle {
                tx,
                hwnd,
                wake_pending,
                thread: Some(thread),
            },
            snapshot,
        )),
        Ok(Err(error)) => Err(error),
        Err(_) => Err("engine did not start in time".into()),
    }
}

type Ready = Sender<Result<(isize, EngineSnapshot), String>>;

fn run(
    config: EngineConfig,
    sink: EventSink,
    rx: Receiver<Command>,
    wake_pending: Arc<AtomicBool>,
    ready: Ready,
) {
    // La sesión de Magnification es de este hilo.
    let magnifier = Magnifier::init().ok();
    let mut core = Core::new(
        WinHardware {
            magnifier,
            vibrance: DriverVibrance::load(),
        },
        config.restore_path,
    );

    // Primero se deshace lo que haya quedado de una sesión anterior; recién
    // después se capturan los originales.
    let found = displays::enumerate().unwrap_or_default();
    let mut recovered = core.recover(&found);
    let mut modes = ResolutionManager::new(config.modes_restore_path);
    for job in modes.recovery_jobs(&found) {
        let _ = execute_job(&job);
        recovered = true;
    }
    // Los modos o el HDR pudieron cambiar: se vuelve a leer.
    let found = if recovered {
        displays::enumerate().unwrap_or(found)
    } else {
        found
    };
    core.set_displays(found);
    let desktop = config.requested.clone();
    core.set_requested(config.requested);
    core.reconcile(false);

    let remote_message = window::register_message(control::REMOTE_MESSAGE_NAME);
    let (job_tx, job_rx) = mpsc::channel();
    let shell = Rc::new(RefCell::new(Shell {
        core,
        modes,
        settings: config.settings,
        sink,
        rx,
        wake_pending,
        remote_message,
        hwnd: 0,
        identify: None,
        displays_dirty: false,
        health_armed: false,
        mode_timer_armed: false,
        hotkeys_suspended: false,
        job_tx,
        job_rx,
        job_reply: None,
        resolution_rev: 0,
        overlay: None,
        crosshair_preview: None,
        desktop,
        game: None,
        session: SessionPlan::default(),
        profile_resolution: None,
        game_timer_armed: false,
        shutdown_reply: None,
    }));

    let handler_shell = Rc::clone(&shell);
    let window = match HiddenWindow::create(
        control::WINDOW_CLASS_NAME,
        Box::new(move |_hwnd, msg, wparam, _lparam| {
            handler_shell
                .try_borrow_mut()
                .ok()
                .and_then(|mut shell| shell.on_message(msg, wparam))
        }),
    ) {
        Ok(window) => window,
        Err(error) => {
            let _ = ready.send(Err(error.to_string()));
            return;
        }
    };
    window.allow_from_lower_integrity(remote_message);
    let hotkey_ok = window
        .register_hotkey(
            HOTKEY_EMERGENCY,
            MOD_CONTROL | MOD_ALT | MOD_SHIFT | MOD_NOREPEAT,
            VK_R,
        )
        .is_ok();
    window.register_session_notifications();
    let foreground = ForegroundHook::install(window.handle(), WM_FOREGROUND).ok();

    if let Ok(mut shell) = shell.try_borrow_mut() {
        shell.hwnd = window.handle();
        shell.after_change(false);
        let toggle_ok = shell.register_toggle_hotkey();
        shell.apply_crosshair();
        let resolution = shell.resolution_state();
        let snapshot = EngineSnapshot {
            displays: shell.core.displays().to_vec(),
            status: shell.core.status().clone(),
            resolution,
            recovered,
            hotkey_registered: hotkey_ok,
            toggle_hotkey_registered: toggle_ok,
        };
        let _ = ready.send(Ok((window.handle(), snapshot)));
    }

    window::run_message_loop();

    drop(foreground);
    window.unregister_hotkey(HOTKEY_EMERGENCY);
    window.unregister_hotkey(HOTKEY_TOGGLE);
    drop(window);
    if let Ok(mut shell) = shell.try_borrow_mut()
        && let Some(reply) = shell.shutdown_reply.take()
    {
        let _ = reply.send(());
    }
}
