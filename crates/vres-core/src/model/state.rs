//! Estado persistente del motor: la sesión (lo que el usuario dejó aplicado)
//! y los valores originales para restaurar.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::color::ramp::GammaRamp;
use crate::planner::Requested;
use crate::store::{from_hex, to_hex};

/// Versión del esquema de `session.json`.
pub const SESSION_SCHEMA: u32 = 1;
/// Versión del esquema de `restore.json`.
pub const RESTORE_SCHEMA: u32 = 1;

/// `session.json`: lo que estaba aplicado, para retomarlo al abrir VRes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SessionState {
    pub schema_version: u32,
    /// `false` mientras VRes corre; `true` tras una salida limpia.
    pub clean_exit: bool,
    pub requested: Requested,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            schema_version: SESSION_SCHEMA,
            clean_exit: true,
            requested: Requested::default(),
        }
    }
}

/// Valores originales de una pantalla.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreDisplay {
    /// Nombre GDI al momento de capturar (puede cambiar entre sesiones).
    pub gdi_name: String,
    /// Rampa original en hexadecimal (`WORD[3][256]`, little-endian).
    pub ramp: String,
    /// Nivel original de la vibrance del driver, si VRes la cambió.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vibrance: Option<i32>,
}

impl RestoreDisplay {
    /// Guarda la rampa original de una pantalla.
    pub fn new(gdi_name: &str, ramp: &GammaRamp) -> Self {
        Self {
            gdi_name: gdi_name.to_owned(),
            ramp: to_hex(&ramp.to_le_bytes()),
            vibrance: None,
        }
    }

    /// Rampa guardada, si es válida.
    pub fn ramp(&self) -> Option<GammaRamp> {
        GammaRamp::from_le_bytes(&from_hex(&self.ramp)?)
    }
}

/// `restore.json`: se escribe **antes** de la primera modificación y se marca
/// limpio cuando todo vuelve al original. Si VRes arranca y lo encuentra
/// sucio, restaura primero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RestoreState {
    pub schema_version: u32,
    /// Hay cambios de VRes aplicados.
    pub dirty: bool,
    /// Proceso que los aplicó.
    pub pid: u32,
    pub app_version: String,
    /// Valores originales por pantalla (identificador estable).
    pub displays: BTreeMap<String, RestoreDisplay>,
}

impl Default for RestoreState {
    fn default() -> Self {
        Self {
            schema_version: RESTORE_SCHEMA,
            dirty: false,
            pid: 0,
            app_version: crate::APP_VERSION.to_owned(),
            displays: BTreeMap::new(),
        }
    }
}

/// Versión del esquema de `restore-modes.json`.
pub const MODE_RESTORE_SCHEMA: u32 = 1;

/// Lo que VRes cambió en una pantalla: resolución o HDR.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ModeRestoreDisplay {
    /// Nombre GDI al momento del cambio (puede cambiar entre sesiones).
    pub gdi_name: String,
    /// Hay un modo temporal de VRes: se vuelve al guardado en Windows.
    pub mode_changed: bool,
    /// Estado de HDR antes de que VRes lo cambiara.
    pub hdr_was_enabled: Option<bool>,
}

impl ModeRestoreDisplay {
    /// No queda nada que restaurar en esta pantalla.
    pub fn is_clean(&self) -> bool {
        !self.mode_changed && self.hdr_was_enabled.is_none()
    }
}

/// `restore-modes.json`: como `restore.json`, pero para resolución y HDR. Se
/// escribe **antes** de cada cambio; si VRes arranca y lo encuentra sucio,
/// restaura primero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ModeRestoreState {
    pub schema_version: u32,
    pub dirty: bool,
    pub pid: u32,
    pub app_version: String,
    /// Por pantalla (identificador estable).
    pub displays: BTreeMap<String, ModeRestoreDisplay>,
}

impl Default for ModeRestoreState {
    fn default() -> Self {
        Self {
            schema_version: MODE_RESTORE_SCHEMA,
            dirty: false,
            pid: 0,
            app_version: crate::APP_VERSION.to_owned(),
            displays: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_display_round_trips_the_ramp() {
        let ramp = GammaRamp::from_fn(|channel, x| x * (0.7 + 0.1 * channel as f64));
        let saved = RestoreDisplay::new(r"\\.\DISPLAY1", &ramp);
        assert_eq!(saved.ramp.len(), 1536 * 2);
        assert_eq!(saved.ramp(), Some(ramp));
    }

    #[test]
    fn broken_hex_is_rejected() {
        let saved = RestoreDisplay {
            gdi_name: String::new(),
            ramp: "zz".into(),
            vibrance: None,
        };
        assert_eq!(saved.ramp(), None);
    }

    #[test]
    fn session_defaults_fill_missing_fields() {
        let session: SessionState = serde_json::from_str("{}").expect("válido");
        assert!(session.clean_exit);
        assert!(session.requested.color_enabled);
    }
}
