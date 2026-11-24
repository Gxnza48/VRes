//! Estado de cada capa del motor, con motivos en forma de código.
//!
//! El motor nunca emite frases: emite un [`ReasonCode`] y la interfaz lo
//! traduce a los tres idiomas.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::model::vibrance::VibranceBackend;

/// Capas que maneja el motor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Layer {
    Matrix,
    Ramp,
    Vibrance,
    Zoom,
    Crosshair,
    Resolution,
    Hdr,
    Ddc,
}

/// Estado de una capa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum LayerStatus {
    /// No hay nada que aplicar.
    Off,
    /// Aplicado y verificado.
    Ok,
    /// Aplicado en parte, o con una limitación conocida.
    Limited,
    /// No se pudo aplicar.
    Failed,
}

/// Por qué una capa está limitada o falló.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ReasonCode {
    /// HDR activo: algunas capas no son confiables.
    HdrActive,
    /// Hay monitores sin tildar: los efectos globales quedan en pausa.
    PartialTargets,
    /// Un juego en pantalla completa exclusiva salta la capa de Windows.
    ExclusiveFullscreen,
    /// La API de color de Windows no está disponible.
    MagUnavailable,
    /// Windows descartó la matriz de color.
    MagDropped,
    /// Otra app (Lupa, filtros de color) usa la matriz de color.
    MagConflict,
    /// La curva pedida excede lo que Windows acepta: se aplicó la más cercana.
    RampOutOfRange,
    /// Windows rechazó la rampa de gamma.
    RampRejected,
    /// Otra app escribe la rampa de gamma (Luz nocturna, f.lux, calibrador).
    RampConflict,
    /// Este monitor no tiene soporte de vibrance del driver.
    VibranceBackendMissing,
    /// La saturación se aproxima con la vibrance del driver.
    VibranceApproximated,
    /// El driver devolvió un error.
    DriverError,
    /// La pantalla ya no está conectada.
    DisplayGone,
    /// Efectos en pausa.
    Paused,
    /// Comparando contra la imagen original.
    CompareHeld,
    /// La función solo actúa con el juego en primer plano.
    NotInGame,
    /// El monitor no admite ese modo.
    ModeNotSupported,
    /// Windows no pudo aplicar el modo.
    ModeChangeFailed,
    /// El driver o el monitor no respetaron el escalado pedido.
    ScalingIgnored,
    /// No se pudo cambiar el HDR.
    HdrChangeFailed,
    /// Otro programa ya usa ese atajo.
    HotkeyTaken,
    /// No se pudo hacer zoom (por ejemplo, con la Lupa de Windows abierta).
    ZoomUnavailable,
    /// VRes arrancó en modo seguro tras un cierre inesperado.
    SafeModeAfterCrash,
}

/// Estado de una capa, para una pantalla o para todo el escritorio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LayerReport {
    pub layer: Layer,
    /// `None` para capas globales (matriz, zoom).
    pub display: Option<String>,
    pub status: LayerStatus,
    pub reason: Option<ReasonCode>,
    /// Detalle técnico en inglés, solo para logs.
    pub detail: Option<String>,
}

/// Vibrance del driver de una pantalla.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DisplayVibrance {
    pub backend: VibranceBackend,
    /// El valor que tenía el driver antes de VRes, en la escala de VRes (0–100).
    pub original: u32,
}

/// Estado completo del motor (evento `engine:status`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EngineStatus {
    /// Aumenta cada vez que cambia algo.
    #[ts(type = "number")]
    pub rev: u64,
    /// Estado de cada capa (y de cada pantalla, para las capas por monitor).
    pub layers: Vec<LayerReport>,
    /// Pantallas con vibrance del driver.
    #[serde(default)]
    pub vibrance: BTreeMap<String, DisplayVibrance>,
}

impl EngineStatus {
    /// El peor estado entre todas las capas (para el indicador "En vivo").
    pub fn overall(&self) -> LayerStatus {
        let rank = |status: LayerStatus| match status {
            LayerStatus::Off => 0,
            LayerStatus::Ok => 1,
            LayerStatus::Limited => 2,
            LayerStatus::Failed => 3,
        };
        self.layers
            .iter()
            .map(|layer| layer.status)
            .max_by_key(|status| rank(*status))
            .unwrap_or(LayerStatus::Off)
    }
}
