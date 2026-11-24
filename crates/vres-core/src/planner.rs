//! El planificador: de lo que pide el usuario a lo que hay que escribir.
//!
//! Es una función **pura, total y determinista**: no toca Windows, así que se
//! prueba en cualquier sistema. El motor (`vres-engine`) aplica el [`Plan`],
//! lo relee y completa el estado de cada capa.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::color::matrix::{self, ColorMatrix, DEFAULT_COEFFICIENT_LIMIT};
use crate::color::ramp::RampParams;
use crate::model::display::DisplayInfo;
use crate::model::look::{DisplayTrim, Look};
use crate::status::{Layer, LayerStatus, ReasonCode};

/// Lo que pide el usuario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct Requested {
    /// Interruptor general del color.
    pub color_enabled: bool,
    /// "Pausar todo" (bandeja o atajo): no cambia la configuración.
    pub paused: bool,
    /// Mientras se mantiene "Comparar", se muestra la imagen original.
    pub compare: bool,
    pub look: Look,
    /// Pantallas tildadas (identificadores estables). `None` = todas.
    pub targets: Option<BTreeSet<String>>,
    /// Ajuste propio de cada pantalla.
    pub trims: BTreeMap<String, DisplayTrim>,
}

impl Default for Requested {
    fn default() -> Self {
        Self {
            color_enabled: true,
            paused: false,
            compare: false,
            look: Look::default(),
            targets: None,
            trims: BTreeMap::new(),
        }
    }
}

impl Requested {
    /// `true` si la pantalla `id` está tildada.
    pub fn targets_display(&self, id: &str) -> bool {
        self.targets
            .as_ref()
            .is_none_or(|targets| targets.contains(id))
    }
}

/// Qué escribir en la rampa de una pantalla.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RampTarget {
    /// La rampa original del monitor (sin cambios de VRes).
    Original,
    /// Una curva propia, que el motor compone sobre la original.
    Custom(RampParams),
}

/// Nota del planificador sobre una capa (limitación conocida de antemano).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanNote {
    pub layer: Layer,
    pub display: Option<String>,
    pub status: LayerStatus,
    pub reason: ReasonCode,
}

/// Lo que el motor tiene que escribir.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    /// Matriz de color global.
    pub matrix: ColorMatrix,
    /// Rampa de cada pantalla, por identificador estable.
    pub ramps: BTreeMap<String, RampTarget>,
    /// Limitaciones conocidas.
    pub notes: Vec<PlanNote>,
}

impl Plan {
    /// Plan que no cambia nada (todo vuelve al original).
    pub fn neutral(displays: &[DisplayInfo]) -> Self {
        Self {
            matrix: ColorMatrix::IDENTITY,
            ramps: displays
                .iter()
                .map(|display| (display.id.clone(), RampTarget::Original))
                .collect(),
            notes: Vec::new(),
        }
    }

    /// `true` si el plan no cambia nada.
    pub fn is_neutral(&self) -> bool {
        self.matrix == ColorMatrix::IDENTITY
            && self
                .ramps
                .values()
                .all(|ramp| matches!(ramp, RampTarget::Original))
    }
}

/// Calcula el plan para las pantallas `displays`.
pub fn plan(requested: &Requested, displays: &[DisplayInfo]) -> Plan {
    let mut plan = Plan::neutral(displays);

    if !requested.color_enabled {
        return plan;
    }
    if requested.paused || requested.compare {
        let reason = if requested.paused {
            ReasonCode::Paused
        } else {
            ReasonCode::CompareHeld
        };
        plan.notes.push(PlanNote {
            layer: Layer::Matrix,
            display: None,
            status: LayerStatus::Off,
            reason,
        });
        return plan;
    }

    let look = requested.look.clamped();

    // Matriz: global. Si hay pantallas sin tildar, se pausa (no se puede
    // aplicar solo a algunas).
    let built = matrix::build_matrix(&look, None, DEFAULT_COEFFICIENT_LIMIT);
    if !built.matrix.is_identity(1e-6) {
        let all_targeted = displays
            .iter()
            .all(|display| requested.targets_display(&display.id));
        if all_targeted {
            plan.matrix = built.matrix;
        } else {
            plan.notes.push(PlanNote {
                layer: Layer::Matrix,
                display: None,
                status: LayerStatus::Limited,
                reason: ReasonCode::PartialTargets,
            });
        }
    }

    // Rampas: por pantalla tildada. Con HDR, Windows no define su
    // comportamiento: se dejan en la original.
    for display in displays {
        if !requested.targets_display(&display.id) {
            continue;
        }
        let trim = requested
            .trims
            .get(&display.id)
            .cloned()
            .unwrap_or_default()
            .clamped();
        let params = RampParams::new(&look.tone, &look.eye_care, &trim);
        if params.is_neutral() {
            continue;
        }
        if display.hdr.enabled {
            plan.notes.push(PlanNote {
                layer: Layer::Ramp,
                display: Some(display.id.clone()),
                status: LayerStatus::Limited,
                reason: ReasonCode::HdrActive,
            });
            continue;
        }
        plan.ramps
            .insert(display.id.clone(), RampTarget::Custom(params));
    }

    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::display::{AdapterInfo, GpuVendor, HdrState, RectPx};
    use crate::model::look::Tone;

    fn display(id: &str, x: i32, hdr: bool) -> DisplayInfo {
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
            hdr: HdrState {
                supported: hdr,
                enabled: hdr,
            },
        }
    }

    fn two_displays() -> Vec<DisplayInfo> {
        vec![display("a", 0, false), display("b", 1920, false)]
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

    #[test]
    fn default_request_is_neutral() {
        let plan = plan(&Requested::default(), &two_displays());
        assert!(plan.is_neutral());
        assert!(plan.notes.is_empty());
    }

    #[test]
    fn vivid_look_applies_everywhere() {
        let plan = plan(&vivid(), &two_displays());
        assert!(!plan.matrix.is_identity(1e-6));
        assert!(
            plan.ramps
                .values()
                .all(|ramp| matches!(ramp, RampTarget::Custom(_)))
        );
    }

    #[test]
    fn disabled_paused_or_compare_is_neutral() {
        for request in [
            Requested {
                color_enabled: false,
                ..vivid()
            },
            Requested {
                paused: true,
                ..vivid()
            },
            Requested {
                compare: true,
                ..vivid()
            },
        ] {
            assert!(plan(&request, &two_displays()).is_neutral(), "{request:?}");
        }
    }

    #[test]
    fn partial_targets_pause_the_matrix_but_keep_ramps() {
        let request = Requested {
            targets: Some(BTreeSet::from(["a".to_owned()])),
            ..vivid()
        };
        let plan = plan(&request, &two_displays());
        assert!(plan.matrix.is_identity(1e-6));
        assert!(matches!(plan.ramps["a"], RampTarget::Custom(_)));
        assert!(matches!(plan.ramps["b"], RampTarget::Original));
        assert!(
            plan.notes
                .iter()
                .any(|note| note.reason == ReasonCode::PartialTargets)
        );
    }

    #[test]
    fn no_targets_means_nothing_applied() {
        let request = Requested {
            targets: Some(BTreeSet::new()),
            ..vivid()
        };
        let plan = plan(&request, &two_displays());
        assert!(plan.is_neutral());
    }

    #[test]
    fn hdr_display_keeps_its_original_ramp() {
        let displays = vec![display("a", 0, true), display("b", 1920, false)];
        let plan = plan(&vivid(), &displays);
        assert!(matches!(plan.ramps["a"], RampTarget::Original));
        assert!(matches!(plan.ramps["b"], RampTarget::Custom(_)));
        assert!(
            plan.notes
                .iter()
                .any(|note| note.reason == ReasonCode::HdrActive)
        );
    }

    #[test]
    fn saturation_only_touches_the_matrix() {
        let request = Requested {
            look: Look {
                saturation: 200.0,
                ..Look::default()
            },
            ..Requested::default()
        };
        let plan = plan(&request, &two_displays());
        assert!(!plan.matrix.is_identity(1e-6));
        assert!(
            plan.ramps
                .values()
                .all(|ramp| matches!(ramp, RampTarget::Original))
        );
    }

    #[test]
    fn requested_round_trips_through_json() {
        let request = Requested {
            targets: Some(BTreeSet::from(["a".to_owned()])),
            ..vivid()
        };
        let json = serde_json::to_string(&request).expect("serializable");
        let back: Requested = serde_json::from_str(&json).expect("deserializable");
        assert_eq!(back, request);
    }
}
