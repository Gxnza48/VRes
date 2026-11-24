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
use vres_core::enhancers::DarkBoost;
use vres_core::model::display::DisplayInfo;
use vres_core::model::state::{RestoreDisplay, RestoreState};
use vres_core::model::vibrance::VibranceInfo;
use vres_core::planner::{self, PlanNote, RampTarget, Requested, VibranceTarget};
use vres_core::status::{
    DisplayVibrance, EngineStatus, Layer, LayerReport, LayerStatus, ReasonCode,
};
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
    /// Rango y valor de la vibrance del driver (`None`: la pantalla no tiene).
    fn vibrance(&mut self, display: &DisplayInfo) -> Option<VibranceInfo> {
        let _ = display;
        None
    }
    /// Escribe un nivel de vibrance del driver.
    fn set_vibrance(&mut self, display: &DisplayInfo, raw: i32) -> Result<(), String> {
        let _ = (display, raw);
        Err("no vibrance backend".into())
    }
    /// Cambiaron las pantallas: olvidar lo que se sabía de ellas.
    fn forget_displays(&mut self) {}
}

/// Clave de las reparaciones y conflictos de la vibrance de una pantalla.
fn vibrance_key(id: &str) -> String {
    format!("vibrance:{id}")
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
    /// Dark Boost activo (mientras se mantiene su tecla).
    dark_boost: Option<DarkBoost>,
    notes: Vec<PlanNote>,
    applied_matrix: ColorMatrix,
    matrix_report: LayerReport,
    applied_ramps: BTreeMap<String, GammaRamp>,
    ramp_reports: BTreeMap<String, LayerReport>,
    repairs: BTreeMap<String, Repairs>,
    conflicted: BTreeSet<String>,
    /// Rango de la vibrance del driver, por pantalla que la tiene.
    vibrance_ranges: BTreeMap<String, VibranceInfo>,
    /// Nivel del driver antes de que VRes lo tocara.
    vibrance_originals: BTreeMap<String, i32>,
    /// Pantallas cuya vibrance tocó VRes desde la última limpieza.
    vibrance_touched: BTreeSet<String>,
    applied_vibrance: BTreeMap<String, i32>,
    vibrance_reports: BTreeMap<String, LayerReport>,
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
            dark_boost: None,
            notes: Vec::new(),
            applied_matrix: ColorMatrix::IDENTITY,
            matrix_report: report(Layer::Matrix, None, LayerStatus::Off, None, None),
            applied_ramps: BTreeMap::new(),
            ramp_reports: BTreeMap::new(),
            repairs: BTreeMap::new(),
            conflicted: BTreeSet::new(),
            vibrance_ranges: BTreeMap::new(),
            vibrance_originals: BTreeMap::new(),
            vibrance_touched: BTreeSet::new(),
            applied_vibrance: BTreeMap::new(),
            vibrance_reports: BTreeMap::new(),
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
            let Some(entry) = saved.displays.get(&display.id) else {
                continue;
            };
            if let Some(ramp) = entry.ramp() {
                self.writes += 1;
                let _ = self.hw.write_ramp(&display.gdi_name, &ramp);
            }
            if let Some(raw) = entry.vibrance {
                self.writes += 1;
                let _ = self.hw.set_vibrance(display, raw);
            }
        }
        self.restore = RestoreState::default();
        self.persist_restore();
        true
    }

    /// Actualiza las pantallas y captura la rampa original de las nuevas.
    pub fn set_displays(&mut self, displays: Vec<DisplayInfo>) {
        self.hw.forget_displays();
        for display in &displays {
            if !self.vibrance_ranges.contains_key(&display.id)
                && let Some(info) = self.hw.vibrance(display)
            {
                // El original se toma una sola vez: si la pantalla vuelve a
                // conectarse con un valor de VRes, se conserva el verdadero.
                self.vibrance_originals
                    .entry(display.id.clone())
                    .or_insert(info.current);
                self.vibrance_ranges.insert(display.id.clone(), info);
            }
        }
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
        self.vibrance_ranges.retain(|id, _| present(id));
        self.applied_vibrance.retain(|id, _| present(id));
        self.vibrance_reports.retain(|id, _| present(id));
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

    /// Prende (`Some`) o apaga Dark Boost (todavía no aplica: llamar a
    /// [`Core::reconcile`]).
    pub fn set_dark_boost(&mut self, boost: Option<DarkBoost>) {
        self.dark_boost = boost;
    }

    /// El hardware, para lo que no pasa por el planificador (el zoom).
    pub fn hardware_mut(&mut self) -> &mut H {
        &mut self.hw
    }

    /// Estado actual.
    pub fn status(&self) -> &EngineStatus {
        &self.status
    }

    /// `true` si VRes tiene algo aplicado (hay que vigilarlo).
    pub fn is_active(&self) -> bool {
        !self.applied_matrix.is_identity(1e-6)
            || !self.applied_ramps.is_empty()
            || !self.applied_vibrance.is_empty()
    }

    /// Vibrance original de las pantallas que VRes modificó (para la
    /// restauración de emergencia desde otro hilo).
    pub fn vibrance_to_restore(&self) -> Vec<(DisplayInfo, i32)> {
        self.applied_vibrance
            .keys()
            .filter_map(|id| {
                let display = self.displays.iter().find(|display| &display.id == id)?;
                Some((display.clone(), *self.vibrance_originals.get(id)?))
            })
            .collect()
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
        let capable: BTreeSet<String> = self.vibrance_ranges.keys().cloned().collect();
        let plan = planner::plan_full(
            &self.requested,
            &self.displays,
            &capable,
            self.dark_boost.as_ref(),
        );
        self.apply_vibrance(&plan.vibrance, force);
        self.apply_ramps(&plan.ramps, force);
        self.apply_matrix(&plan.matrix, force);
        self.notes = plan.notes;
        self.clean_if_nothing_applied();
        self.refresh_status()
    }

    fn clean_if_nothing_applied(&mut self) {
        if self.applied_ramps.is_empty() && self.applied_vibrance.is_empty() && self.restore.dirty {
            self.mark_clean();
        }
    }

    /// Escribe un nivel de vibrance y lo relee.
    fn write_vibrance_verified(&mut self, display: &DisplayInfo, raw: i32) -> Result<(), String> {
        self.writes += 1;
        self.hw.set_vibrance(display, raw)?;
        match self.hw.vibrance(display) {
            Some(info) if info.current == raw => Ok(()),
            Some(info) => Err(format!("read back {} instead of {raw}", info.current)),
            None => Err("vibrance unavailable after the write".into()),
        }
    }

    fn apply_vibrance(&mut self, targets: &BTreeMap<String, VibranceTarget>, force: bool) {
        for display in self.displays.clone() {
            let id = display.id.clone();
            let Some(range) = self.vibrance_ranges.get(&id).copied() else {
                continue;
            };
            let original = self
                .vibrance_originals
                .get(&id)
                .copied()
                .unwrap_or(range.current);
            let desired = match targets.get(&id) {
                Some(VibranceTarget::Level(value)) => Some(range.raw_for(*value)),
                _ => None,
            };
            let applied = self.applied_vibrance.get(&id).copied();
            if desired == applied && !(force && desired.is_some()) {
                continue;
            }
            match desired {
                None => {
                    if applied.is_some() {
                        self.writes += 1;
                        let _ = self.hw.set_vibrance(&display, original);
                        self.applied_vibrance.remove(&id);
                    }
                    self.vibrance_reports.insert(
                        id.clone(),
                        report(Layer::Vibrance, Some(&id), LayerStatus::Off, None, None),
                    );
                }
                Some(_) if self.conflicted.contains(&vibrance_key(&id)) => {}
                Some(raw) => {
                    // El original se guarda en disco ANTES de modificar nada.
                    self.mark_vibrance_dirty(&id);
                    let entry = match self.write_vibrance_verified(&display, raw) {
                        Ok(()) => {
                            self.applied_vibrance.insert(id.clone(), raw);
                            report(Layer::Vibrance, Some(&id), LayerStatus::Ok, None, None)
                        }
                        Err(error) => {
                            self.applied_vibrance.remove(&id);
                            self.writes += 1;
                            let _ = self.hw.set_vibrance(&display, original);
                            report(
                                Layer::Vibrance,
                                Some(&id),
                                LayerStatus::Failed,
                                Some(ReasonCode::DriverError),
                                Some(error),
                            )
                        }
                    };
                    self.vibrance_reports.insert(id.clone(), entry);
                }
            }
        }
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

        let applied: Vec<(String, i32)> = self
            .applied_vibrance
            .iter()
            .map(|(id, raw)| (id.clone(), *raw))
            .collect();
        for (id, raw) in applied {
            let Some(display) = self
                .displays
                .iter()
                .find(|display| display.id == id)
                .cloned()
            else {
                continue;
            };
            let Some(info) = self.hw.vibrance(&display) else {
                continue;
            };
            if info.current == raw {
                continue;
            }
            let key = vibrance_key(&id);
            if self.count_repair(&key) > MAX_REPAIRS_PER_MINUTE {
                // Otro programa (el panel de la placa, un perfil de juego) la cambia.
                self.applied_vibrance.remove(&id);
                self.conflicted.insert(key);
                self.vibrance_reports.insert(
                    id.clone(),
                    report(
                        Layer::Vibrance,
                        Some(&id),
                        LayerStatus::Limited,
                        Some(ReasonCode::DriverError),
                        Some("another program keeps changing the vibrance".into()),
                    ),
                );
            } else {
                let _ = self.write_vibrance_verified(&display, raw);
            }
        }

        self.clean_if_nothing_applied();
        self.refresh_status()
    }

    /// Cuenta una reparación de `key` en el último minuto.
    fn count_repair(&mut self, key: &str) -> u32 {
        let now = Instant::now();
        let repairs = self.repairs.entry(key.to_owned()).or_insert(Repairs {
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
        repairs.count
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
        for (display, raw) in self.vibrance_to_restore() {
            self.writes += 1;
            let _ = self.hw.set_vibrance(&display, raw);
        }
        self.applied_vibrance.clear();
        for (id, entry) in self.vibrance_reports.iter_mut() {
            *entry = report(Layer::Vibrance, Some(id), LayerStatus::Off, None, None);
        }
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

    /// Como `mark_dirty`, y además guarda el original de la vibrance de `id`.
    fn mark_vibrance_dirty(&mut self, id: &str) {
        if self.vibrance_touched.insert(id.to_owned()) || !self.restore.dirty {
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
        for id in &self.vibrance_touched {
            let Some(&raw) = self.vibrance_originals.get(id) else {
                continue;
            };
            let gdi_name = self
                .displays
                .iter()
                .find(|display| &display.id == id)
                .map(|display| display.gdi_name.clone())
                .unwrap_or_default();
            self.restore
                .displays
                .entry(id.clone())
                .or_insert_with(|| RestoreDisplay {
                    gdi_name,
                    ramp: String::new(),
                    vibrance: None,
                })
                .vibrance = Some(raw);
        }
        self.persist_restore();
    }

    fn mark_clean(&mut self) {
        self.restore = RestoreState::default();
        self.vibrance_touched.clear();
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

            let note = self.notes.iter().find(|note| {
                note.layer == Layer::Vibrance
                    && note.display.as_deref() == Some(display.id.as_str())
            });
            let vibrance = self.vibrance_reports.get(&display.id).cloned();
            if vibrance.is_some()
                || note.is_some()
                || self.vibrance_ranges.contains_key(&display.id)
            {
                let mut entry = vibrance.unwrap_or_else(|| {
                    report(
                        Layer::Vibrance,
                        Some(&display.id),
                        LayerStatus::Off,
                        None,
                        None,
                    )
                });
                if let Some(note) = note
                    && matches!(entry.status, LayerStatus::Off | LayerStatus::Ok)
                {
                    entry.status = note.status;
                    entry.reason = Some(note.reason);
                }
                layers.push(entry);
            }
        }

        let vibrance: BTreeMap<String, DisplayVibrance> = self
            .vibrance_ranges
            .iter()
            .map(|(id, info)| {
                let original = self
                    .vibrance_originals
                    .get(id)
                    .copied()
                    .unwrap_or(info.current);
                (
                    id.clone(),
                    DisplayVibrance {
                        backend: info.backend,
                        original: info.value_for(original).round() as u32,
                    },
                )
            })
            .collect();

        if layers == self.status.layers && vibrance == self.status.vibrance {
            return false;
        }
        self.status.layers = layers;
        self.status.vibrance = vibrance;
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
        vibrance: HashMap<String, VibranceInfo>,
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
        fn vibrance(&mut self, display: &DisplayInfo) -> Option<VibranceInfo> {
            self.vibrance.get(&display.gdi_name).copied()
        }
        fn set_vibrance(&mut self, display: &DisplayInfo, raw: i32) -> Result<(), String> {
            let info = self
                .vibrance
                .get_mut(&display.gdi_name)
                .ok_or_else(|| "no backend".to_owned())?;
            info.current = raw.clamp(info.min, info.max);
            Ok(())
        }
    }

    /// Una pantalla AMD con la vibrance del driver al máximo (como la del usuario).
    fn amd_at_max(hw: &mut FakeHardware, id: &str) {
        hw.vibrance.insert(
            format!(r"\\.\{id}"),
            VibranceInfo {
                backend: vres_core::model::vibrance::VibranceBackend::Amd,
                current: 200,
                min: 0,
                max: 200,
                default: 100,
            },
        );
    }

    fn with_vibrance(value: f32) -> Requested {
        Requested {
            look: Look {
                vibrance: value,
                ..Look::default()
            },
            ..Requested::default()
        }
    }

    #[test]
    fn driver_vibrance_is_applied_verified_and_restored() {
        let mut hw = FakeHardware::default();
        amd_at_max(&mut hw, "a");
        let mut core = core(hw, None);
        core.set_requested(with_vibrance(30.0));
        core.reconcile(false);
        assert_eq!(core.hw.vibrance[r"\\.\a"].current, 60, "30 → 60 en 0–200");
        assert!(core.is_active());
        let entry = core
            .status()
            .layers
            .iter()
            .find(|layer| layer.layer == Layer::Vibrance)
            .expect("capa de vibrance");
        assert_eq!(entry.status, LayerStatus::Ok);
        assert_eq!(
            core.status().vibrance["a"].original,
            100,
            "200 es 100 en la escala de VRes"
        );

        core.set_requested(Requested::default());
        core.reconcile(false);
        assert_eq!(
            core.hw.vibrance[r"\\.\a"].current, 200,
            "vuelve al original del usuario"
        );
        assert!(!core.is_active());
    }

    #[test]
    fn vibrance_original_is_on_disk_before_the_first_write() {
        let path = temp_path("vibrance");
        let mut hw = FakeHardware::default();
        amd_at_max(&mut hw, "a");
        let mut core = core(hw, Some(path.clone()));
        core.set_requested(with_vibrance(80.0));
        core.reconcile(false);
        let saved = match store::read_json::<RestoreState>(&path) {
            Loaded::Ok(saved) => saved,
            other => panic!("restore.json: {other:?}"),
        };
        assert!(saved.dirty);
        assert_eq!(saved.displays["a"].vibrance, Some(200));

        core.restore_all();
        assert_eq!(core.hw.vibrance[r"\\.\a"].current, 200);
    }

    #[test]
    fn a_dirty_session_gets_its_vibrance_back() {
        let path = temp_path("vibrance-recover");
        let saved = RestoreState {
            dirty: true,
            displays: [(
                "a".to_owned(),
                RestoreDisplay {
                    gdi_name: r"\\.\a".into(),
                    ramp: String::new(),
                    vibrance: Some(200),
                },
            )]
            .into(),
            ..RestoreState::default()
        };
        store::write_json_atomic(&path, &saved).expect("restore.json");
        let mut hw = FakeHardware::default();
        amd_at_max(&mut hw, "a");
        hw.vibrance.get_mut(r"\\.\a").expect("pantalla").current = 140;
        let mut core = core(hw, Some(path));
        assert!(core.recover(&[display("a", 0)]));
        assert_eq!(core.hw.vibrance[r"\\.\a"].current, 200);
    }

    #[test]
    fn vibrance_changed_by_another_program_is_repaired_then_left_alone() {
        let mut hw = FakeHardware::default();
        amd_at_max(&mut hw, "a");
        let mut core = core(hw, None);
        core.set_requested(with_vibrance(70.0));
        core.reconcile(false);
        for _ in 0..MAX_REPAIRS_PER_MINUTE {
            core.hw
                .vibrance
                .get_mut(r"\\.\a")
                .expect("pantalla")
                .current = 10;
            core.health_check();
            assert_eq!(core.hw.vibrance[r"\\.\a"].current, 140, "se repara");
        }
        core.hw
            .vibrance
            .get_mut(r"\\.\a")
            .expect("pantalla")
            .current = 10;
        core.health_check();
        assert_eq!(core.hw.vibrance[r"\\.\a"].current, 10, "deja de pelear");
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
    fn dark_boost_lifts_shadows_while_held_even_with_color_off() {
        let mut core = core(FakeHardware::default(), None);
        core.set_requested(Requested {
            color_enabled: false,
            ..Requested::default()
        });
        core.reconcile(false);
        assert!(!core.is_active());
        let original = core.hw.ramps.get(r"\\.\a").cloned().unwrap_or_default();

        core.set_dark_boost(Some(DarkBoost::default()));
        core.reconcile(false);
        assert!(core.is_active(), "las sombras se levantan");
        assert_ne!(core.hw.ramps[r"\\.\a"], original);

        core.set_dark_boost(None);
        core.reconcile(false);
        assert!(!core.is_active());
        assert_eq!(
            core.hw.ramps[r"\\.\a"], original,
            "vuelve la rampa original"
        );
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
