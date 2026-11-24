//! Núcleo del motor: reconcilia lo pedido con lo aplicado.
//!
//! Es independiente de Windows: habla con el hardware a través del trait
//! [`Hardware`], así que se prueba con un hardware falso. El hilo del motor
//! (en `lib.rs`) solo traduce mensajes de Windows en llamadas a este núcleo.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use vres_core::color::matrix::ColorMatrix;
use vres_core::color::ramp::{GammaRamp, RampRule, build_ramp};
use vres_core::model::display::DisplayInfo;
use vres_core::model::state::{RestoreDisplay, RestoreState};
use vres_core::planner::{self, PlanNote, RampTarget, Requested};
use vres_core::status::{EngineStatus, Layer, LayerReport, LayerStatus, ReasonCode};
use vres_core::store::{self, Loaded};

/// Tolerancia al releer una rampa: un escalón de 8 bits.
pub const RAMP_TOLERANCE: u16 = 0x0101;
/// Tolerancia al releer la matriz.
const MATRIX_TOLERANCE: f32 = 1e-4;
/// Si otra app pisa una rampa más veces que esto por minuto, VRes deja de
/// pelear y avisa.
const MAX_REPAIRS_PER_MINUTE: u32 = 3;

/// Operaciones de hardware que necesita el motor.
pub trait Hardware {
    fn set_matrix(&mut self, matrix: &ColorMatrix) -> Result<(), String>;
    fn matrix(&mut self) -> Result<ColorMatrix, String>;
    fn read_ramp(&mut self, gdi_name: &str) -> Result<GammaRamp, String>;
    /// Devuelve `true` si Windows dijo que la aceptó.
    fn write_ramp(&mut self, gdi_name: &str, ramp: &GammaRamp) -> Result<bool, String>;
}

#[derive(Debug, Clone)]
struct Original {
    gdi_name: String,
    ramp: GammaRamp,
}

#[derive(Debug, Clone, Copy)]
struct Repairs {
    since: Instant,
    count: u32,
}

fn report(
    layer: Layer,
    display: Option<&str>,
    status: LayerStatus,
    reason: Option<ReasonCode>,
    detail: Option<String>,
) -> LayerReport {
    LayerReport {
        layer,
        display: display.map(str::to_owned),
        status,
        reason,
        detail,
    }
}

/// El núcleo del motor.
pub struct Core<H: Hardware> {
    hw: H,
    restore_path: Option<PathBuf>,
    restore: RestoreState,
    displays: Vec<DisplayInfo>,
    originals: BTreeMap<String, Original>,
    requested: Requested,
    notes: Vec<PlanNote>,
    applied_matrix: ColorMatrix,
    matrix_report: LayerReport,
    applied_ramps: BTreeMap<String, GammaRamp>,
    ramp_reports: BTreeMap<String, LayerReport>,
    repairs: BTreeMap<String, Repairs>,
    conflicted: BTreeSet<String>,
    status: EngineStatus,
    /// Escrituras de hardware realizadas (para pruebas y diagnóstico).
    pub writes: u64,
}

impl<H: Hardware> Core<H> {
    /// Crea el núcleo. `restore_path` es `restore.json` (`None` en pruebas).
    pub fn new(hw: H, restore_path: Option<PathBuf>) -> Self {
        Self {
            hw,
            restore_path,
            restore: RestoreState::default(),
            displays: Vec::new(),
            originals: BTreeMap::new(),
            requested: Requested::default(),
            notes: Vec::new(),
            applied_matrix: ColorMatrix::IDENTITY,
            matrix_report: report(Layer::Matrix, None, LayerStatus::Off, None, None),
            applied_ramps: BTreeMap::new(),
            ramp_reports: BTreeMap::new(),
            repairs: BTreeMap::new(),
            conflicted: BTreeSet::new(),
            status: EngineStatus::default(),
            writes: 0,
        }
    }

    /// Si una sesión anterior dejó cambios (`restore.json` sucio), los deshace
    /// escribiendo las rampas originales guardadas. Devuelve `true` si había
    /// algo que restaurar. Se llama **antes** de capturar originales.
    pub fn recover(&mut self, displays: &[DisplayInfo]) -> bool {
        let Some(path) = &self.restore_path else {
            return false;
        };
        let saved = match store::read_json::<RestoreState>(path) {
            Loaded::Ok(saved) => saved,
            _ => return false,
        };
        if !saved.dirty {
            return false;
        }
        for display in displays {
            if let Some(ramp) = saved
                .displays
                .get(&display.id)
                .and_then(RestoreDisplay::ramp)
            {
                self.writes += 1;
                let _ = self.hw.write_ramp(&display.gdi_name, &ramp);
            }
        }
        self.restore = RestoreState::default();
        self.persist_restore();
        true
    }

    /// Actualiza las pantallas y captura la rampa original de las nuevas.
    pub fn set_displays(&mut self, displays: Vec<DisplayInfo>) {
        for display in &displays {
            match self.originals.get_mut(&display.id) {
                // El nombre GDI puede cambiar (por ejemplo, al reconectar).
                Some(original) => original.gdi_name.clone_from(&display.gdi_name),
                None => {
                    if let Ok(ramp) = self.hw.read_ramp(&display.gdi_name) {
                        self.originals.insert(
                            display.id.clone(),
                            Original {
                                gdi_name: display.gdi_name.clone(),
                                ramp,
                            },
                        );
                    }
                }
            }
        }
        let present = |id: &String| displays.iter().any(|display| &display.id == id);
        self.applied_ramps.retain(|id, _| present(id));
        self.ramp_reports.retain(|id, _| present(id));
        self.displays = displays;
        if self.restore.dirty {
            self.persist_dirty_marker();
        }
    }

    /// Pantallas actuales.
    pub fn displays(&self) -> &[DisplayInfo] {
        &self.displays
    }

    /// Lo que está pedido.
    pub fn requested(&self) -> &Requested {
        &self.requested
    }

    /// Cambia lo pedido (todavía no aplica: llamar a [`Core::reconcile`]).
    pub fn set_requested(&mut self, requested: Requested) {
        if requested != self.requested {
            // Un cambio del usuario da otra oportunidad a las pantallas en conflicto.
            self.conflicted.clear();
            self.repairs.clear();
        }
        self.requested = requested;
    }

    /// Estado actual.
    pub fn status(&self) -> &EngineStatus {
        &self.status
    }

    /// `true` si VRes tiene algo aplicado (hay que vigilarlo).
    pub fn is_active(&self) -> bool {
        !self.applied_matrix.is_identity(1e-6) || !self.applied_ramps.is_empty()
    }

    /// Rampas originales de las pantallas que VRes modificó (para la
    /// restauración de emergencia desde otro hilo).
    pub fn originals_to_restore(&self) -> Vec<(String, GammaRamp)> {
        self.applied_ramps
            .keys()
            .filter_map(|id| self.originals.get(id))
            .map(|original| (original.gdi_name.clone(), original.ramp.clone()))
            .collect()
    }

    /// Aplica lo pedido. Escribe solo lo que cambió (salvo `force`, que
    /// reescribe todo: tras un cambio de pantalla o al desbloquear la sesión).
    /// Devuelve `true` si cambió el estado.
    pub fn reconcile(&mut self, force: bool) -> bool {
        let plan = planner::plan(&self.requested, &self.displays);
        self.apply_ramps(&plan.ramps, force);
        self.apply_matrix(&plan.matrix, force);
        self.notes = plan.notes;
        if self.applied_ramps.is_empty() && self.restore.dirty {
            self.mark_clean();
        }
        self.refresh_status()
    }

    fn write_verified(&mut self, gdi_name: &str, ramp: &GammaRamp) -> Result<bool, String> {
        self.writes += 1;
        if !self.hw.write_ramp(gdi_name, ramp)? {
            return Ok(false);
        }
        let current = self.hw.read_ramp(gdi_name)?;
        Ok(current.max_abs_diff(ramp) <= RAMP_TOLERANCE)
    }

    fn apply_ramps(&mut self, targets: &BTreeMap<String, RampTarget>, force: bool) {
        let ids: Vec<String> = self.displays.iter().map(|d| d.id.clone()).collect();
        for id in ids {
            let Some(original) = self.originals.get(&id).cloned() else {
                self.ramp_reports.insert(
                    id.clone(),
                    report(
                        Layer::Ramp,
                        Some(&id),
                        LayerStatus::Failed,
                        Some(ReasonCode::DriverError),
                        Some("original ramp could not be read".into()),
                    ),
                );
                continue;
            };
            let target = targets.get(&id).copied().unwrap_or(RampTarget::Original);
            let (desired, kept) = match target {
                RampTarget::Original => (None, 1.0),
                RampTarget::Custom(params) => {
                    let built = build_ramp(&params, &original.ramp, RampRule::VRES);
                    (Some(built.ramp), built.kept)
                }
            };

            let unchanged = match (&desired, self.applied_ramps.get(&id)) {
                (None, None) => true,
                (Some(wanted), Some(applied)) => wanted == applied,
                _ => false,
            };
            if unchanged && !force {
                continue;
            }

            match desired {
                None => {
                    if self.applied_ramps.remove(&id).is_some() || force {
                        self.writes += 1;
                        let _ = self.hw.write_ramp(&original.gdi_name, &original.ramp);
                    }
                    self.ramp_reports.insert(
                        id.clone(),
                        report(Layer::Ramp, Some(&id), LayerStatus::Off, None, None),
                    );
                }
                Some(_) if self.conflicted.contains(&id) => {}
                Some(ramp) => {
                    // El original se guarda en disco ANTES de modificar nada.
                    self.mark_dirty();
                    let result = self.write_verified(&original.gdi_name, &ramp);
                    let entry = match result {
                        Ok(true) => {
                            self.applied_ramps.insert(id.clone(), ramp);
                            if kept < 0.999 {
                                report(
                                    Layer::Ramp,
                                    Some(&id),
                                    LayerStatus::Limited,
                                    Some(ReasonCode::RampOutOfRange),
                                    Some(format!(
                                        "kept {:.0}% of the requested curve",
                                        kept * 100.0
                                    )),
                                )
                            } else {
                                report(Layer::Ramp, Some(&id), LayerStatus::Ok, None, None)
                            }
                        }
                        Ok(false) | Err(_) => {
                            // Se deja el monitor como estaba.
                            self.applied_ramps.remove(&id);
                            self.writes += 1;
                            let _ = self.hw.write_ramp(&original.gdi_name, &original.ramp);
                            let detail = result.err();
                            report(
                                Layer::Ramp,
                                Some(&id),
                                LayerStatus::Failed,
                                Some(ReasonCode::RampRejected),
                                detail,
                            )
                        }
                    };
                    self.ramp_reports.insert(id.clone(), entry);
                }
            }
        }
    }

    fn apply_matrix(&mut self, desired: &ColorMatrix, force: bool) {
        if *desired == self.applied_matrix && !force {
            return;
        }
        self.writes += 1;
        let result = self.hw.set_matrix(desired).and_then(|()| self.hw.matrix());
        self.matrix_report = match result {
            Ok(current) if current.max_abs_diff(desired) <= MATRIX_TOLERANCE => {
                self.applied_matrix = *desired;
                let status = if desired.is_identity(1e-6) {
                    LayerStatus::Off
                } else {
                    LayerStatus::Ok
                };
                report(Layer::Matrix, None, status, None, None)
            }
            Ok(_) => {
                self.applied_matrix = ColorMatrix::IDENTITY;
                report(
                    Layer::Matrix,
                    None,
                    LayerStatus::Failed,
                    Some(ReasonCode::MagDropped),
                    Some("readback differs".into()),
                )
            }
            Err(error) => {
                self.applied_matrix = ColorMatrix::IDENTITY;
                report(
                    Layer::Matrix,
                    None,
                    LayerStatus::Failed,
                    Some(ReasonCode::MagUnavailable),
                    Some(error),
                )
            }
        };
    }

    /// Relee lo aplicado y lo repara si Windows u otra app lo cambió.
    /// Devuelve `true` si cambió el estado.
    pub fn health_check(&mut self) -> bool {
        if !self.applied_matrix.is_identity(1e-6) {
            match self.hw.matrix() {
                Ok(current) if current.max_abs_diff(&self.applied_matrix) <= MATRIX_TOLERANCE => {}
                Ok(current) if current.is_identity(1e-6) => {
                    // Windows la descartó: se reaplica.
                    let applied = self.applied_matrix;
                    self.apply_matrix(&applied, true);
                }
                Ok(_) => {
                    // Otra app (Lupa, filtros de color) es la dueña: no se pelea.
                    self.applied_matrix = ColorMatrix::IDENTITY;
                    self.matrix_report = report(
                        Layer::Matrix,
                        None,
                        LayerStatus::Limited,
                        Some(ReasonCode::MagConflict),
                        None,
                    );
                }
                Err(error) => {
                    self.matrix_report = report(
                        Layer::Matrix,
                        None,
                        LayerStatus::Failed,
                        Some(ReasonCode::MagUnavailable),
                        Some(error),
                    );
                }
            }
        }

        let applied: Vec<(String, GammaRamp)> = self
            .applied_ramps
            .iter()
            .map(|(id, ramp)| (id.clone(), ramp.clone()))
            .collect();
        for (id, ramp) in applied {
            let Some(original) = self.originals.get(&id).cloned() else {
                continue;
            };
            let Ok(current) = self.hw.read_ramp(&original.gdi_name) else {
                continue;
            };
            if current.max_abs_diff(&ramp) <= RAMP_TOLERANCE {
                continue;
            }
            let now = Instant::now();
            let repairs = self.repairs.entry(id.clone()).or_insert(Repairs {
                since: now,
                count: 0,
            });
            if now.duration_since(repairs.since) > Duration::from_secs(60) {
                *repairs = Repairs {
                    since: now,
                    count: 0,
                };
            }
            repairs.count += 1;
            if repairs.count > MAX_REPAIRS_PER_MINUTE {
                // Otra app escribe la rampa (Luz nocturna, f.lux, un calibrador).
                self.applied_ramps.remove(&id);
                self.conflicted.insert(id.clone());
                self.ramp_reports.insert(
                    id.clone(),
                    report(
                        Layer::Ramp,
                        Some(&id),
                        LayerStatus::Limited,
                        Some(ReasonCode::RampConflict),
                        None,
                    ),
                );
            } else {
                let _ = self.write_verified(&original.gdi_name, &ramp);
            }
        }

        if self.applied_ramps.is_empty() && self.restore.dirty {
            self.mark_clean();
        }
        self.refresh_status()
    }

    /// Vuelve todo al original (salida, fin de sesión, emergencia).
    pub fn restore_all(&mut self) {
        let ids: Vec<String> = self.applied_ramps.keys().cloned().collect();
        for id in ids {
            if let Some(original) = self.originals.get(&id).cloned() {
                self.writes += 1;
                let _ = self.hw.write_ramp(&original.gdi_name, &original.ramp);
            }
        }
        self.applied_ramps.clear();
        if !self.applied_matrix.is_identity(1e-6) {
            self.writes += 1;
            let _ = self.hw.set_matrix(&ColorMatrix::IDENTITY);
        }
        self.applied_matrix = ColorMatrix::IDENTITY;
        self.matrix_report = report(Layer::Matrix, None, LayerStatus::Off, None, None);
        for (id, entry) in self.ramp_reports.iter_mut() {
            *entry = report(Layer::Ramp, Some(id), LayerStatus::Off, None, None);
        }
        self.mark_clean();
        self.refresh_status();
    }

    fn mark_dirty(&mut self) {
        if !self.restore.dirty {
            self.restore.dirty = true;
            self.persist_dirty_marker();
        }
    }

    fn persist_dirty_marker(&mut self) {
        self.restore.pid = std::process::id();
        self.restore.app_version = vres_core::APP_VERSION.to_owned();
        self.restore.displays = self
            .originals
            .iter()
            .map(|(id, original)| {
                (
                    id.clone(),
                    RestoreDisplay::new(&original.gdi_name, &original.ramp),
                )
            })
            .collect();
        self.persist_restore();
    }

    fn mark_clean(&mut self) {
        self.restore = RestoreState::default();
        self.persist_restore();
    }

    fn persist_restore(&self) {
        if let Some(path) = &self.restore_path {
            let _ = store::write_json_atomic(path, &self.restore);
        }
    }

    fn refresh_status(&mut self) -> bool {
        let mut matrix = self.matrix_report.clone();
        if let Some(note) = self.notes.iter().find(|note| note.layer == Layer::Matrix)
            && matches!(matrix.status, LayerStatus::Off)
        {
            matrix.status = note.status;
            matrix.reason = Some(note.reason);
        }

        let mut layers = vec![matrix];
        for display in &self.displays {
            let mut entry = self
                .ramp_reports
                .get(&display.id)
                .cloned()
                .unwrap_or_else(|| {
                    report(Layer::Ramp, Some(&display.id), LayerStatus::Off, None, None)
                });
            if let Some(note) = self.notes.iter().find(|note| {
                note.layer == Layer::Ramp && note.display.as_deref() == Some(display.id.as_str())
            }) && matches!(entry.status, LayerStatus::Off)
            {
                entry.status = note.status;
                entry.reason = Some(note.reason);
            }
            layers.push(entry);
        }

        if layers == self.status.layers {
            return false;
        }
        self.status.layers = layers;
        self.status.rev += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use vres_core::model::display::{AdapterInfo, GpuVendor, HdrState, RectPx};
    use vres_core::model::look::{Look, Tone};

    #[derive(Default)]
    struct FakeHardware {
        matrix: Option<ColorMatrix>,
        ramps: HashMap<String, GammaRamp>,
        reject_ramps: bool,
        matrix_unavailable: bool,
    }

    impl Hardware for FakeHardware {
        fn set_matrix(&mut self, matrix: &ColorMatrix) -> Result<(), String> {
            if self.matrix_unavailable {
                return Err("unavailable".into());
            }
            self.matrix = Some(*matrix);
            Ok(())
        }
        fn matrix(&mut self) -> Result<ColorMatrix, String> {
            Ok(self.matrix.unwrap_or(ColorMatrix::IDENTITY))
        }
        fn read_ramp(&mut self, gdi_name: &str) -> Result<GammaRamp, String> {
            Ok(self.ramps.get(gdi_name).cloned().unwrap_or_default())
        }
        fn write_ramp(&mut self, gdi_name: &str, ramp: &GammaRamp) -> Result<bool, String> {
            if self.reject_ramps && *ramp != GammaRamp::identity() {
                return Ok(false);
            }
            self.ramps.insert(gdi_name.to_owned(), ramp.clone());
            Ok(true)
        }
    }

    fn display(id: &str, x: i32) -> DisplayInfo {
        DisplayInfo {
            id: id.to_owned(),
            gdi_name: format!(r"\\.\{id}"),
            friendly_name: id.to_owned(),
            adapter: AdapterInfo {
                vendor: GpuVendor::Other,
                name: String::new(),
            },
            bounds: RectPx {
                x,
                y: 0,
                width: 1920,
                height: 1080,
            },
            primary: x == 0,
            internal: false,
            refresh_hz: 60.0,
            hdr: HdrState::default(),
        }
    }

    fn vivid() -> Requested {
        Requested {
            look: Look {
                saturation: 160.0,
                tone: Tone {
                    gamma: 1.2,
                    ..Tone::default()
                },
                ..Look::default()
            },
            ..Requested::default()
        }
    }

    fn core(hw: FakeHardware, path: Option<PathBuf>) -> Core<FakeHardware> {
        let mut core = Core::new(hw, path);
        core.set_displays(vec![display("a", 0), display("b", 1920)]);
        core
    }

    fn temp_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vres-engine-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("restore.json")
    }

    #[test]
    fn neutral_request_writes_nothing() {
        let mut core = core(FakeHardware::default(), None);
        core.reconcile(false);
        assert_eq!(core.writes, 0);
        assert!(!core.is_active());
    }

    #[test]
    fn vivid_look_is_applied_and_verified() {
        let mut core = core(FakeHardware::default(), None);
        core.set_requested(vivid());
        assert!(core.reconcile(false));
        assert!(core.is_active());
        assert!(
            core.status()
                .layers
                .iter()
                .all(|layer| layer.status == LayerStatus::Ok),
            "{:?}",
            core.status()
        );
    }

    #[test]
    fn repeated_requests_do_not_rewrite() {
        let mut core = core(FakeHardware::default(), None);
        core.set_requested(vivid());
        core.reconcile(false);
        let after_first = core.writes;
        for _ in 0..100 {
            core.set_requested(vivid());
            core.reconcile(false);
        }
        assert_eq!(core.writes, after_first);
    }

    #[test]
    fn restore_file_is_dirty_before_the_first_ramp_write() {
        let path = temp_path("dirty");
        let mut core = core(FakeHardware::default(), Some(path.clone()));
        core.set_requested(vivid());
        core.reconcile(false);
        let saved = match store::read_json::<RestoreState>(&path) {
            Loaded::Ok(saved) => saved,
            other => panic!("restore.json: {other:?}"),
        };
        assert!(saved.dirty);
        assert_eq!(saved.displays.len(), 2);

        core.restore_all();
        let saved = match store::read_json::<RestoreState>(&path) {
            Loaded::Ok(saved) => saved,
            other => panic!("restore.json: {other:?}"),
        };
        assert!(!saved.dirty);
        let _ = std::fs::remove_dir_all(path.parent().expect("carpeta"));
    }

    #[test]
    fn rejected_ramp_is_reported_and_original_kept() {
        let hw = FakeHardware {
            reject_ramps: true,
            ..FakeHardware::default()
        };
        let mut core = core(hw, None);
        core.set_requested(vivid());
        core.reconcile(false);
        let ramp_layers: Vec<_> = core
            .status()
            .layers
            .iter()
            .filter(|layer| layer.layer == Layer::Ramp)
            .collect();
        assert!(
            ramp_layers
                .iter()
                .all(|layer| layer.reason == Some(ReasonCode::RampRejected))
        );
        assert!(core.originals_to_restore().is_empty());
    }

    #[test]
    fn restore_all_puts_back_originals() {
        let mut core = core(FakeHardware::default(), None);
        core.set_requested(vivid());
        core.reconcile(false);
        core.restore_all();
        assert!(!core.is_active());
        assert_eq!(core.hw.ramps[r"\\.\a"], GammaRamp::identity());
        assert_eq!(core.hw.matrix, Some(ColorMatrix::IDENTITY));
    }

    #[test]
    fn dropped_matrix_is_reapplied() {
        let mut core = core(FakeHardware::default(), None);
        core.set_requested(vivid());
        core.reconcile(false);
        core.hw.matrix = Some(ColorMatrix::IDENTITY);
        core.health_check();
        assert!(!core.hw.matrix.expect("matrix").is_identity(1e-6));
    }

    #[test]
    fn ramp_fought_by_another_app_ends_in_conflict() {
        let mut core = core(FakeHardware::default(), None);
        core.set_requested(vivid());
        core.reconcile(false);
        for _ in 0..=MAX_REPAIRS_PER_MINUTE {
            core.hw.ramps.insert(r"\\.\a".into(), GammaRamp::identity());
            core.health_check();
        }
        let layer = core
            .status()
            .layers
            .iter()
            .find(|layer| layer.display.as_deref() == Some("a"))
            .cloned()
            .expect("capa de a");
        assert_eq!(layer.reason, Some(ReasonCode::RampConflict));
    }

    #[test]
    fn partial_targets_pause_the_matrix() {
        let mut core = core(FakeHardware::default(), None);
        core.set_requested(Requested {
            targets: Some(BTreeSet::from(["a".to_owned()])),
            ..vivid()
        });
        core.reconcile(false);
        let matrix = &core.status().layers[0];
        assert_eq!(matrix.status, LayerStatus::Limited);
        assert_eq!(matrix.reason, Some(ReasonCode::PartialTargets));
    }

    #[test]
    fn unavailable_matrix_is_reported() {
        let hw = FakeHardware {
            matrix_unavailable: true,
            ..FakeHardware::default()
        };
        let mut core = core(hw, None);
        core.set_requested(vivid());
        core.reconcile(false);
        assert_eq!(
            core.status().layers[0].reason,
            Some(ReasonCode::MagUnavailable)
        );
    }

    #[test]
    fn recover_restores_a_dirty_session() {
        let path = temp_path("recover");
        let mut saved = RestoreState {
            dirty: true,
            ..RestoreState::default()
        };
        saved.displays.insert(
            "a".into(),
            RestoreDisplay::new(r"\\.\a", &GammaRamp::identity()),
        );
        store::write_json_atomic(&path, &saved).expect("restore.json");

        let mut hw = FakeHardware::default();
        hw.ramps
            .insert(r"\\.\a".into(), GammaRamp::from_fn(|_, x| x * 0.5));
        let mut core = Core::new(hw, Some(path.clone()));
        assert!(core.recover(&[display("a", 0)]));
        assert_eq!(core.hw.ramps[r"\\.\a"], GammaRamp::identity());
        assert!(!core.recover(&[display("a", 0)]), "ya quedó limpio");
        let _ = std::fs::remove_dir_all(path.parent().expect("carpeta"));
    }
}
