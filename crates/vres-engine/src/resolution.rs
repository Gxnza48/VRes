//! Cambios de resolución y de HDR: qué hacer y en qué orden.
//!
//! Este módulo no llama a Windows. Decide trabajos ([`ModeJob`]) que el hilo
//! del motor ejecuta en un hilo auxiliar (un cambio de modo puede tardar
//! segundos) y recibe su resultado con [`ResolutionManager::finish`]. Así se
//! prueba con un reloj y un hardware falsos.
//!
//! Reglas:
//! - antes de cada cambio se marca `restore-modes.json` como sucio;
//! - un modo nuevo desde la interfaz queda **pendiente**: si nadie lo confirma
//!   a tiempo, vuelve solo;
//! - el atajo solo alterna a modos ya confirmados, sin cuenta regresiva;
//! - un trabajo a la vez.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use vres_core::ipc::ErrorCode;
use vres_core::model::display::DisplayInfo;
use vres_core::model::resolution::{ModeOutcome, ModeRequest, ModeResolved, PendingMode};
use vres_core::model::state::{ModeRestoreDisplay, ModeRestoreState};
use vres_core::settings::SavedMode;
use vres_core::status::ReasonCode;
use vres_core::store::{self, Loaded};

/// Un trabajo para el hilo auxiliar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModeJob {
    /// Aplicar un modo temporal (y pedir su escalado).
    Apply {
        gdi_name: String,
        request: ModeRequest,
    },
    /// Volver al modo guardado en Windows.
    Revert { gdi_name: String },
    /// Prender o apagar HDR.
    SetHdr { gdi_name: String, enabled: bool },
}

/// Resultado de un trabajo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobResult {
    /// Hecho. `reason` informa una limitación (por ejemplo, escalado ignorado).
    Done { reason: Option<ReasonCode> },
    /// Falló; `detail` es técnico, para los logs.
    Failed { detail: String },
}

/// Qué hacer con el usuario cuando termina un trabajo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Feedback {
    /// Nada en particular.
    None,
    /// Tono ascendente (se activó la estirada con el atajo).
    ToneUp,
    /// Tono descendente (se volvió a la de Windows con el atajo).
    ToneDown,
}

/// Lo que pasó al terminar un trabajo (o al confirmar).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Outcome {
    /// Respuesta para quien pidió el cambio desde la interfaz.
    pub reply: Option<Result<Option<PendingMode>, ErrorCode>>,
    /// Cambio pendiente de confirmación recién creado.
    pub pending: Option<PendingMode>,
    /// Un cambio terminó.
    pub resolved: Option<ModeResolved>,
    /// Hay que releer los modos y avisarle a la interfaz.
    pub changed: bool,
    /// Sonido para el atajo.
    pub feedback: Option<Feedback>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Purpose {
    /// Probar un modo desde la interfaz: queda pendiente.
    Try { token: String, seconds: u32 },
    /// Alternar con el atajo a un modo confirmado.
    ToggleOn,
    /// Un perfil de juego pide un modo confirmado (sin tonos).
    Profile,
    /// Volver al modo anterior del pendiente (pedido o vencido).
    RevertPending { expired: bool },
    /// Volver a la resolución de Windows.
    RevertDisplay { from_hotkey: bool },
    /// HDR.
    Hdr { enabled: bool },
}

#[derive(Debug, Clone)]
struct InFlight {
    display_id: String,
    gdi_name: String,
    job: ModeJob,
    purpose: Purpose,
}

#[derive(Debug, Clone)]
struct Pending {
    token: String,
    display_id: String,
    gdi_name: String,
    request: ModeRequest,
    deadline: Instant,
    seconds: u32,
    /// Modo de VRes que había antes (para volver a él y no a la nativa).
    previous: Option<ModeRequest>,
    reason: Option<ReasonCode>,
}

/// Decide los cambios de resolución y HDR.
#[derive(Debug)]
pub struct ResolutionManager {
    /// Modos temporales de VRes aplicados, por pantalla.
    changed: BTreeMap<String, ModeRequest>,
    pending: Option<Pending>,
    in_flight: Option<InFlight>,
    restore: ModeRestoreState,
    restore_path: Option<PathBuf>,
    next_token: u64,
    salt: u64,
}

impl ResolutionManager {
    /// Crea el administrador. `restore_path` es `restore-modes.json` (`None`
    /// en pruebas).
    pub fn new(restore_path: Option<PathBuf>) -> Self {
        // Los tokens no son secretos: solo evitan confirmar un cambio viejo.
        let salt = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos() as u64)
            .unwrap_or(0)
            ^ u64::from(std::process::id()).rotate_left(32);
        Self {
            changed: BTreeMap::new(),
            pending: None,
            in_flight: None,
            restore: ModeRestoreState::default(),
            restore_path,
            next_token: 1,
            salt,
        }
    }

    /// Trabajos para deshacer lo que dejó una sesión que no cerró bien. Marca
    /// el archivo como limpio: quien llama los ejecuta enseguida.
    pub fn recovery_jobs(&mut self, displays: &[DisplayInfo]) -> Vec<ModeJob> {
        let Some(path) = &self.restore_path else {
            return Vec::new();
        };
        let saved = match store::read_json::<ModeRestoreState>(path) {
            Loaded::Ok(saved) if saved.dirty => saved,
            _ => return Vec::new(),
        };
        let jobs = jobs_to_restore(&saved.displays, displays);
        self.restore = ModeRestoreState::default();
        self.persist();
        jobs
    }

    /// `true` si hay un trabajo en curso.
    pub fn busy(&self) -> bool {
        self.in_flight.is_some()
    }

    /// Modo temporal de VRes en esa pantalla, si hay.
    pub fn changed(&self, display_id: &str) -> Option<&ModeRequest> {
        self.changed.get(display_id)
    }

    /// Cambio pendiente de confirmación, con el tiempo que le queda.
    pub fn pending(&self, now: Instant) -> Option<PendingMode> {
        self.pending
            .as_ref()
            .map(|pending| pending_info(pending, now))
    }

    /// `true` si hay algo que restaurar (modos o HDR).
    pub fn has_changes(&self) -> bool {
        self.restore.dirty
    }

    fn new_token(&mut self) -> String {
        let token = format!("{:016x}", self.salt.wrapping_add(self.next_token));
        self.next_token += 1;
        token
    }

    fn check_free(&self) -> Result<(), ErrorCode> {
        if self.in_flight.is_some() || self.pending.is_some() {
            Err(ErrorCode::ModeBusy)
        } else {
            Ok(())
        }
    }

    fn launch(&mut self, display: &DisplayInfo, job: ModeJob, purpose: Purpose) -> ModeJob {
        self.in_flight = Some(InFlight {
            display_id: display.id.clone(),
            gdi_name: display.gdi_name.clone(),
            job: job.clone(),
            purpose,
        });
        job
    }

    /// Probar un modo desde la interfaz. `listed` indica si el monitor lo
    /// admite. Queda pendiente de confirmación durante `seconds`.
    pub fn start_try(
        &mut self,
        display: &DisplayInfo,
        request: ModeRequest,
        listed: bool,
        seconds: u32,
    ) -> Result<ModeJob, ErrorCode> {
        self.check_free()?;
        if !listed {
            return Err(ErrorCode::ModeNotSupported);
        }
        self.mark_mode_changed(display);
        let token = self.new_token();
        let job = ModeJob::Apply {
            gdi_name: display.gdi_name.clone(),
            request,
        };
        Ok(self.launch(display, job, Purpose::Try { token, seconds }))
    }

    /// El usuario conservó el modo pendiente.
    pub fn confirm(&mut self, token: &str) -> Result<ModeResolved, ErrorCode> {
        match &self.pending {
            Some(pending) if pending.token == token && self.in_flight.is_none() => {}
            _ => return Err(ErrorCode::ModeExpired),
        }
        let pending = self.pending.take().ok_or(ErrorCode::ModeExpired)?;
        Ok(ModeResolved {
            token: pending.token,
            display_id: pending.display_id,
            mode: pending.request,
            outcome: ModeOutcome::Kept,
            reason: pending.reason,
        })
    }

    fn revert_pending_job(&mut self, expired: bool) -> Option<ModeJob> {
        let pending = self.pending.as_ref()?;
        let job = match pending.previous {
            Some(previous) => ModeJob::Apply {
                gdi_name: pending.gdi_name.clone(),
                request: previous,
            },
            None => ModeJob::Revert {
                gdi_name: pending.gdi_name.clone(),
            },
        };
        self.in_flight = Some(InFlight {
            display_id: pending.display_id.clone(),
            gdi_name: pending.gdi_name.clone(),
            job: job.clone(),
            purpose: Purpose::RevertPending { expired },
        });
        Some(job)
    }

    /// El usuario pidió volver atrás.
    pub fn start_revert(&mut self, token: &str) -> Result<ModeJob, ErrorCode> {
        match &self.pending {
            Some(pending) if pending.token == token && self.in_flight.is_none() => {}
            _ => return Err(ErrorCode::ModeExpired),
        }
        self.revert_pending_job(false).ok_or(ErrorCode::ModeExpired)
    }

    /// Si la cuenta regresiva venció, el trabajo para volver atrás.
    pub fn tick(&mut self, now: Instant) -> Option<ModeJob> {
        let due = self
            .pending
            .as_ref()
            .is_some_and(|pending| now >= pending.deadline);
        if due && self.in_flight.is_none() {
            self.revert_pending_job(true)
        } else {
            None
        }
    }

    /// Volver a la resolución de Windows en una pantalla. `None` si VRes no
    /// tenía nada aplicado ahí.
    pub fn start_restore_display(
        &mut self,
        display: &DisplayInfo,
        from_hotkey: bool,
    ) -> Result<Option<ModeJob>, ErrorCode> {
        self.check_free()?;
        if !self.changed.contains_key(&display.id) {
            return Ok(None);
        }
        let job = ModeJob::Revert {
            gdi_name: display.gdi_name.clone(),
        };
        Ok(Some(self.launch(
            display,
            job,
            Purpose::RevertDisplay { from_hotkey },
        )))
    }

    /// El atajo: si la pantalla del destino tiene un modo de VRes, vuelve a la
    /// de Windows; si no, aplica el destino (que debe estar confirmado).
    pub fn start_toggle(
        &mut self,
        target: &SavedMode,
        confirmed: bool,
        display: &DisplayInfo,
        listed: bool,
    ) -> Result<ModeJob, ErrorCode> {
        self.check_free()?;
        if self.changed.contains_key(&display.id) {
            return self
                .start_restore_display(display, true)?
                .ok_or(ErrorCode::ModeFailed);
        }
        if !confirmed {
            return Err(ErrorCode::ModeNotConfirmed);
        }
        if !listed {
            return Err(ErrorCode::ModeNotSupported);
        }
        self.mark_mode_changed(display);
        let job = ModeJob::Apply {
            gdi_name: display.gdi_name.clone(),
            request: target.mode,
        };
        Ok(self.launch(display, job, Purpose::ToggleOn))
    }

    /// Un perfil de juego pide un modo ya confirmado. `None` si ya está
    /// aplicado (no hace nada).
    pub fn start_ensure(
        &mut self,
        target: &SavedMode,
        display: &DisplayInfo,
        listed: bool,
    ) -> Result<Option<ModeJob>, ErrorCode> {
        self.check_free()?;
        if self.changed.get(&display.id) == Some(&target.mode) {
            return Ok(None);
        }
        if !listed {
            return Err(ErrorCode::ModeNotSupported);
        }
        self.mark_mode_changed(display);
        let job = ModeJob::Apply {
            gdi_name: display.gdi_name.clone(),
            request: target.mode,
        };
        Ok(Some(self.launch(display, job, Purpose::Profile)))
    }

    /// Prender o apagar HDR. `None` si ya está así.
    pub fn start_hdr(
        &mut self,
        display: &DisplayInfo,
        enabled: bool,
    ) -> Result<Option<ModeJob>, ErrorCode> {
        if self.in_flight.is_some() {
            return Err(ErrorCode::ModeBusy);
        }
        if !display.hdr.supported {
            return Err(ErrorCode::HdrFailed);
        }
        if display.hdr.enabled == enabled {
            return Ok(None);
        }
        let entry = self.restore.displays.entry(display.id.clone()).or_default();
        entry.gdi_name = display.gdi_name.clone();
        if entry.hdr_was_enabled.is_none() {
            entry.hdr_was_enabled = Some(display.hdr.enabled);
        }
        self.restore.dirty = true;
        self.persist_dirty();
        let job = ModeJob::SetHdr {
            gdi_name: display.gdi_name.clone(),
            enabled,
        };
        Ok(Some(self.launch(display, job, Purpose::Hdr { enabled })))
    }

    /// Terminó el trabajo en curso.
    pub fn finish(&mut self, result: JobResult, now: Instant) -> Outcome {
        let Some(flight) = self.in_flight.take() else {
            return Outcome::default();
        };
        let mut outcome = Outcome {
            changed: true,
            ..Outcome::default()
        };
        let ok_reason = match &result {
            JobResult::Done { reason } => Some(*reason),
            JobResult::Failed { .. } => None,
        };

        match flight.purpose {
            Purpose::Try { token, seconds } => {
                let ModeJob::Apply { request, .. } = flight.job else {
                    return outcome;
                };
                match ok_reason {
                    Some(reason) => {
                        let previous = self.changed.insert(flight.display_id.clone(), request);
                        let pending = Pending {
                            token,
                            display_id: flight.display_id,
                            gdi_name: flight.gdi_name,
                            request,
                            deadline: now + Duration::from_secs(u64::from(seconds)),
                            seconds,
                            previous,
                            reason,
                        };
                        let info = pending_info(&pending, now);
                        self.pending = Some(pending);
                        outcome.pending = Some(info.clone());
                        outcome.reply = Some(Ok(Some(info)));
                    }
                    None => {
                        // Windows no lo aplicó: no queda nada que revertir.
                        self.unmark_mode_if_unchanged(&flight.display_id);
                        outcome.reply = Some(Err(ErrorCode::ModeFailed));
                        outcome.resolved = Some(ModeResolved {
                            token,
                            display_id: flight.display_id,
                            mode: request,
                            outcome: ModeOutcome::Failed,
                            reason: Some(ReasonCode::ModeChangeFailed),
                        });
                    }
                }
            }
            Purpose::RevertPending { expired } => {
                let Some(pending) = self.pending.take() else {
                    return outcome;
                };
                let kind = if expired {
                    ModeOutcome::Expired
                } else {
                    ModeOutcome::Reverted
                };
                if ok_reason.is_some() {
                    match pending.previous {
                        Some(previous) => {
                            self.changed.insert(pending.display_id.clone(), previous);
                        }
                        None => {
                            self.changed.remove(&pending.display_id);
                            self.unmark_mode_if_unchanged(&pending.display_id);
                        }
                    }
                    outcome.reply = Some(Ok(None));
                } else {
                    // No se pudo volver: el modo queda aplicado y se restaura al salir.
                    outcome.reply = Some(Err(ErrorCode::ModeFailed));
                }
                outcome.resolved = Some(ModeResolved {
                    token: pending.token,
                    display_id: pending.display_id,
                    mode: pending.request,
                    outcome: if ok_reason.is_some() {
                        kind
                    } else {
                        ModeOutcome::Failed
                    },
                    reason: ok_reason.is_none().then_some(ReasonCode::ModeChangeFailed),
                });
            }
            Purpose::ToggleOn | Purpose::Profile => {
                let tone = flight.purpose == Purpose::ToggleOn;
                if let ModeJob::Apply { request, .. } = flight.job
                    && ok_reason.is_some()
                {
                    self.changed.insert(flight.display_id, request);
                    if tone {
                        outcome.feedback = Some(Feedback::ToneUp);
                    }
                } else {
                    self.unmark_mode_if_unchanged(&flight.display_id);
                }
            }
            Purpose::RevertDisplay { from_hotkey } => {
                if ok_reason.is_some() {
                    self.changed.remove(&flight.display_id);
                    self.unmark_mode_if_unchanged(&flight.display_id);
                    outcome.reply = Some(Ok(None));
                    if from_hotkey {
                        outcome.feedback = Some(Feedback::ToneDown);
                    }
                } else {
                    outcome.reply = Some(Err(ErrorCode::ModeFailed));
                }
            }
            Purpose::Hdr { enabled } => {
                if ok_reason.is_some() {
                    // Si quedó como estaba antes de VRes, no hay nada que restaurar.
                    if let Some(entry) = self.restore.displays.get_mut(&flight.display_id)
                        && entry.hdr_was_enabled == Some(enabled)
                    {
                        entry.hdr_was_enabled = None;
                    }
                    self.clean_if_nothing_left();
                    outcome.reply = Some(Ok(None));
                } else {
                    outcome.reply = Some(Err(ErrorCode::HdrFailed));
                }
            }
        }
        outcome
    }

    /// Cambiaron las pantallas: se olvidan las que ya no están.
    pub fn displays_changed(&mut self, displays: &[DisplayInfo]) -> Option<ModeResolved> {
        let present = |id: &str| displays.iter().any(|display| display.id == id);
        self.changed.retain(|id, _| present(id));
        let gone = self
            .pending
            .as_ref()
            .is_some_and(|pending| !present(&pending.display_id));
        if gone && self.in_flight.is_none() {
            let pending = self.pending.take()?;
            return Some(ModeResolved {
                token: pending.token,
                display_id: pending.display_id,
                mode: pending.request,
                outcome: ModeOutcome::Failed,
                reason: Some(ReasonCode::DisplayGone),
            });
        }
        None
    }

    /// Trabajos que dejarían todo como estaba, sin olvidar nada (para la
    /// restauración de emergencia desde otro hilo).
    pub fn emergency_jobs(&self, displays: &[DisplayInfo]) -> Vec<ModeJob> {
        jobs_to_restore(&self.restore.displays, displays)
    }

    /// Trabajos para dejar todo como estaba (al salir o con el atajo de
    /// emergencia). Olvida lo pendiente; quien llama los ejecuta enseguida y
    /// después llama a [`ResolutionManager::restored`].
    pub fn restore_jobs(&mut self, displays: &[DisplayInfo]) -> Vec<ModeJob> {
        self.pending = None;
        self.in_flight = None;
        self.changed.clear();
        jobs_to_restore(&self.restore.displays, displays)
    }

    /// Ya se ejecutaron los trabajos de [`ResolutionManager::restore_jobs`].
    pub fn restored(&mut self) {
        self.restore = ModeRestoreState::default();
        self.persist();
    }

    fn mark_mode_changed(&mut self, display: &DisplayInfo) {
        let entry = self.restore.displays.entry(display.id.clone()).or_default();
        entry.gdi_name = display.gdi_name.clone();
        entry.mode_changed = true;
        self.restore.dirty = true;
        self.persist_dirty();
    }

    fn unmark_mode_if_unchanged(&mut self, display_id: &str) {
        if self.changed.contains_key(display_id) {
            return;
        }
        if let Some(entry) = self.restore.displays.get_mut(display_id) {
            entry.mode_changed = false;
        }
        self.clean_if_nothing_left();
    }

    fn clean_if_nothing_left(&mut self) {
        self.restore
            .displays
            .retain(|_, entry: &mut ModeRestoreDisplay| !entry.is_clean());
        if self.restore.displays.is_empty() && self.restore.dirty {
            self.restore = ModeRestoreState::default();
            self.persist();
        } else if self.restore.dirty {
            self.persist_dirty();
        }
    }

    fn persist_dirty(&mut self) {
        self.restore.pid = std::process::id();
        self.restore.app_version = vres_core::APP_VERSION.to_owned();
        self.persist();
    }

    fn persist(&self) {
        if let Some(path) = &self.restore_path {
            let _ = store::write_json_atomic(path, &self.restore);
        }
    }
}

fn pending_info(pending: &Pending, now: Instant) -> PendingMode {
    PendingMode {
        token: pending.token.clone(),
        display_id: pending.display_id.clone(),
        mode: pending.request,
        expires_in_ms: pending.deadline.saturating_duration_since(now).as_millis() as u64,
        seconds: pending.seconds,
        reason: pending.reason,
    }
}

/// Trabajos para restaurar lo guardado, usando el nombre GDI actual de cada
/// pantalla (puede haber cambiado desde que se guardó).
fn jobs_to_restore(
    saved: &BTreeMap<String, ModeRestoreDisplay>,
    displays: &[DisplayInfo],
) -> Vec<ModeJob> {
    let mut jobs = Vec::new();
    for (id, entry) in saved {
        let gdi_name = displays
            .iter()
            .find(|display| &display.id == id)
            .map(|display| display.gdi_name.clone())
            .unwrap_or_else(|| entry.gdi_name.clone());
        if entry.mode_changed {
            jobs.push(ModeJob::Revert {
                gdi_name: gdi_name.clone(),
            });
        }
        if let Some(enabled) = entry.hdr_was_enabled {
            jobs.push(ModeJob::SetHdr { gdi_name, enabled });
        }
    }
    jobs
}

#[cfg(test)]
mod tests {
    use super::*;
    use vres_core::model::display::{AdapterInfo, GpuVendor, HdrState, RectPx};
    use vres_core::model::resolution::Scaling;

    fn display(id: &str) -> DisplayInfo {
        DisplayInfo {
            id: id.into(),
            gdi_name: format!(r"\\.\{id}"),
            friendly_name: id.into(),
            adapter: AdapterInfo {
                vendor: GpuVendor::Nvidia,
                name: String::new(),
            },
            bounds: RectPx {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            primary: true,
            internal: false,
            refresh_hz: 200.0,
            hdr: HdrState {
                supported: true,
                enabled: false,
            },
        }
    }

    fn stretched(width: u32) -> ModeRequest {
        ModeRequest {
            width,
            height: 1080,
            refresh_hz: 200,
            scaling: Scaling::Stretched,
        }
    }

    fn ok() -> JobResult {
        JobResult::Done { reason: None }
    }

    fn temp_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vres-modes-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("restore-modes.json")
    }

    fn saved_state(path: &std::path::Path) -> ModeRestoreState {
        match store::read_json::<ModeRestoreState>(path) {
            Loaded::Ok(state) => state,
            other => panic!("restore-modes.json: {other:?}"),
        }
    }

    #[test]
    fn a_tried_mode_waits_for_confirmation_and_can_be_kept() {
        let mut manager = ResolutionManager::new(None);
        let screen = display("A");
        let now = Instant::now();
        let job = manager
            .start_try(&screen, stretched(1440), true, 15)
            .expect("libre");
        assert!(matches!(job, ModeJob::Apply { .. }));
        assert!(manager.busy());

        let outcome = manager.finish(ok(), now);
        let pending = outcome.pending.expect("pendiente");
        assert_eq!(pending.expires_in_ms, 15_000);
        assert_eq!(outcome.reply, Some(Ok(Some(pending.clone()))));
        assert!(manager.changed("A").is_some());

        let kept = manager.confirm(&pending.token).expect("token válido");
        assert_eq!(kept.outcome, ModeOutcome::Kept);
        assert!(manager.pending(now).is_none());
        assert_eq!(manager.confirm(&pending.token), Err(ErrorCode::ModeExpired));
    }

    #[test]
    fn an_unanswered_change_reverts_by_itself() {
        let mut manager = ResolutionManager::new(None);
        let screen = display("A");
        let start = Instant::now();
        manager
            .start_try(&screen, stretched(1440), true, 10)
            .expect("libre");
        manager.finish(ok(), start);

        assert_eq!(manager.tick(start + Duration::from_secs(9)), None);
        let job = manager
            .tick(start + Duration::from_secs(10))
            .expect("vence");
        assert_eq!(
            job,
            ModeJob::Revert {
                gdi_name: screen.gdi_name.clone()
            }
        );
        let outcome = manager.finish(ok(), start + Duration::from_secs(11));
        assert_eq!(
            outcome.resolved.map(|r| r.outcome),
            Some(ModeOutcome::Expired)
        );
        assert!(manager.changed("A").is_none());
        assert!(!manager.has_changes(), "todo volvió al original");
    }

    #[test]
    fn reverting_goes_back_to_the_previous_vres_mode() {
        let mut manager = ResolutionManager::new(None);
        let screen = display("A");
        let now = Instant::now();
        manager
            .start_try(&screen, stretched(1440), true, 15)
            .expect("ok");
        let first = manager.finish(ok(), now).pending.expect("ok");
        manager.confirm(&first.token).expect("ok");

        manager
            .start_try(&screen, stretched(1280), true, 15)
            .expect("ok");
        let second = manager.finish(ok(), now).pending.expect("ok");
        let job = manager.start_revert(&second.token).expect("ok");
        assert_eq!(
            job,
            ModeJob::Apply {
                gdi_name: screen.gdi_name.clone(),
                request: stretched(1440)
            }
        );
        manager.finish(ok(), now);
        assert_eq!(manager.changed("A"), Some(&stretched(1440)));
    }

    #[test]
    fn invalid_tokens_and_busy_states_are_errors() {
        let mut manager = ResolutionManager::new(None);
        let screen = display("A");
        assert_eq!(manager.start_revert("nada"), Err(ErrorCode::ModeExpired));
        assert_eq!(
            manager.start_try(&screen, stretched(1350), false, 15),
            Err(ErrorCode::ModeNotSupported)
        );
        manager
            .start_try(&screen, stretched(1440), true, 15)
            .expect("ok");
        assert_eq!(
            manager.start_try(&screen, stretched(1280), true, 15),
            Err(ErrorCode::ModeBusy),
            "un trabajo a la vez"
        );
        let pending = manager.finish(ok(), Instant::now()).pending.expect("ok");
        assert_eq!(manager.confirm("otro"), Err(ErrorCode::ModeExpired));
        assert_eq!(
            manager.start_try(&screen, stretched(1280), true, 15),
            Err(ErrorCode::ModeBusy),
            "primero hay que responder al pendiente"
        );
        manager.confirm(&pending.token).expect("ok");
    }

    #[test]
    fn a_failed_change_leaves_nothing_to_restore() {
        let path = temp_path("failed");
        let mut manager = ResolutionManager::new(Some(path.clone()));
        let screen = display("A");
        manager
            .start_try(&screen, stretched(1440), true, 15)
            .expect("ok");
        assert!(saved_state(&path).dirty, "sucio antes de cambiar");

        let outcome = manager.finish(
            JobResult::Failed {
                detail: "BADMODE".into(),
            },
            Instant::now(),
        );
        assert_eq!(outcome.reply, Some(Err(ErrorCode::ModeFailed)));
        assert_eq!(
            outcome.resolved.map(|r| r.outcome),
            Some(ModeOutcome::Failed)
        );
        assert!(!saved_state(&path).dirty);
    }

    #[test]
    fn the_hotkey_only_uses_confirmed_modes_and_toggles_back() {
        let mut manager = ResolutionManager::new(None);
        let screen = display("A");
        let target = SavedMode {
            display_id: "A".into(),
            mode: stretched(1440),
        };
        assert_eq!(
            manager.start_toggle(&target, false, &screen, true),
            Err(ErrorCode::ModeNotConfirmed)
        );

        manager
            .start_toggle(&target, true, &screen, true)
            .expect("ok");
        let on = manager.finish(ok(), Instant::now());
        assert_eq!(on.feedback, Some(Feedback::ToneUp));
        assert!(on.pending.is_none(), "sin cuenta regresiva");

        let job = manager
            .start_toggle(&target, true, &screen, true)
            .expect("ok");
        assert!(matches!(job, ModeJob::Revert { .. }));
        let off = manager.finish(ok(), Instant::now());
        assert_eq!(off.feedback, Some(Feedback::ToneDown));
        assert!(manager.changed("A").is_none());
    }

    #[test]
    fn a_profile_ensures_its_mode_once() {
        let mut manager = ResolutionManager::new(None);
        let screen = display("A");
        let target = SavedMode {
            display_id: "A".into(),
            mode: stretched(1440),
        };
        let job = manager
            .start_ensure(&target, &screen, true)
            .expect("libre")
            .expect("hay que aplicarlo");
        assert!(matches!(job, ModeJob::Apply { .. }));
        let outcome = manager.finish(ok(), Instant::now());
        assert_eq!(outcome.feedback, None, "los perfiles no suenan");
        assert_eq!(manager.changed("A"), Some(&stretched(1440)));
        assert_eq!(
            manager.start_ensure(&target, &screen, true),
            Ok(None),
            "ya está"
        );
        assert_eq!(
            manager.start_ensure(
                &SavedMode {
                    display_id: "A".into(),
                    mode: stretched(1280)
                },
                &screen,
                false
            ),
            Err(ErrorCode::ModeNotSupported)
        );
    }

    #[test]
    fn hdr_original_is_saved_and_cleared_when_back() {
        let path = temp_path("hdr");
        let mut manager = ResolutionManager::new(Some(path.clone()));
        let mut screen = display("A");
        manager
            .start_hdr(&screen, true)
            .expect("ok")
            .expect("hay que cambiar");
        assert_eq!(
            saved_state(&path).displays["A"].hdr_was_enabled,
            Some(false)
        );
        manager.finish(ok(), Instant::now());

        screen.hdr.enabled = true;
        manager
            .start_hdr(&screen, false)
            .expect("ok")
            .expect("hay que cambiar");
        manager.finish(ok(), Instant::now());
        assert!(!saved_state(&path).dirty, "volvió a como estaba");
    }

    #[test]
    fn restore_and_recovery_revert_modes_and_hdr() {
        let path = temp_path("recover");
        let screen = display("A");
        {
            let mut manager = ResolutionManager::new(Some(path.clone()));
            manager
                .start_try(&screen, stretched(1440), true, 15)
                .expect("ok");
            manager.finish(ok(), Instant::now());
            manager.start_hdr(&screen, true).expect("ok");
            // El proceso "muere" acá: el trabajo de HDR nunca termina.
        }
        let mut renamed = screen.clone();
        renamed.gdi_name = r"\\.\DISPLAY9".into();
        let mut manager = ResolutionManager::new(Some(path.clone()));
        let jobs = manager.recovery_jobs(std::slice::from_ref(&renamed));
        assert_eq!(
            jobs,
            vec![
                ModeJob::Revert {
                    gdi_name: renamed.gdi_name.clone()
                },
                ModeJob::SetHdr {
                    gdi_name: renamed.gdi_name.clone(),
                    enabled: false
                },
            ]
        );
        assert!(!saved_state(&path).dirty);
        assert!(manager.recovery_jobs(&[renamed]).is_empty(), "una sola vez");
    }

    #[test]
    fn a_disconnected_display_drops_its_pending_change() {
        let mut manager = ResolutionManager::new(None);
        let screen = display("A");
        manager
            .start_try(&screen, stretched(1440), true, 15)
            .expect("ok");
        manager.finish(ok(), Instant::now());
        let resolved = manager.displays_changed(&[display("B")]).expect("se fue");
        assert_eq!(resolved.reason, Some(ReasonCode::DisplayGone));
        assert!(manager.changed("A").is_none());
    }
}
