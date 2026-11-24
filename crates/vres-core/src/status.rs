//! Estado de cada capa del motor, con motivos en forma de código.
//!
//! El motor nunca emite frases: emite un [`ReasonCode`] y la interfaz lo
//! traduce a los tres idiomas.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

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
    /// Otro programa ya usa ese atajo.
    HotkeyTaken,
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
