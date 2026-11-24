//! Tipos que cruzan la frontera entre el motor (Rust) y la interfaz (React).
//!
//! Regla: la interfaz nunca escribe estos tipos a mano. Se generan con ts-rs en
//! `src/lib/bindings` y CI falla si quedaron desactualizados.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::model::display::DisplayInfo;
use crate::planner::Requested;
use crate::status::EngineStatus;

/// Avisos para el usuario (evento `app:notice` y al arrancar).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Notice {
    /// Había cambios de una sesión que no cerró bien: se restauraron.
    Recovered,
    /// VRes arrancó con el color apagado tras un cierre inesperado.
    SafeMode,
    /// Se restableció la pantalla (atajo de emergencia o pedido externo).
    EmergencyReset,
    /// Otro programa usa el atajo de emergencia.
    HotkeyTaken,
    /// El motor no pudo arrancar.
    EngineUnavailable,
    /// Una resolución nueva volvió sola porque nadie la confirmó.
    ModeExpired,
    /// Otro programa usa el atajo de resolución.
    ToggleHotkeyTaken,
}

/// Datos que la interfaz pide una sola vez, al arrancar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Bootstrap {
    /// Versión instalada de VRes.
    pub app_version: String,
    /// Idioma del sistema (BCP 47, por ejemplo `es-AR`), si se pudo detectar.
    pub system_language: Option<String>,
    /// `true` si VRes arrancó solo con Windows (sin abrir la ventana).
    pub autostarted: bool,
    /// Pantallas activas.
    pub displays: Vec<DisplayInfo>,
    /// Lo que está pedido (look, pantallas tildadas, interruptores).
    pub requested: Requested,
    /// Estado de cada capa.
    pub status: EngineStatus,
    /// Avisos pendientes (recuperación, modo seguro…).
    pub notices: Vec<Notice>,
}

/// Una actualización disponible en GitHub Releases.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateInfo {
    /// Versión nueva.
    pub version: String,
    /// Versión instalada.
    pub current_version: String,
    /// Notas de la versión, si las hay.
    pub notes: Option<String>,
    /// Fecha de publicación (RFC 3339), si la hay.
    pub date: Option<String>,
}

/// Progreso de descarga de una actualización (evento `update:progress`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UpdateProgress {
    /// Bytes descargados hasta ahora.
    #[ts(type = "number")]
    pub downloaded: u64,
    /// Tamaño total, si el servidor lo informó.
    #[ts(type = "number | null")]
    pub total: Option<u64>,
}

/// Código estable de error. La interfaz traduce por código, nunca por mensaje.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ErrorCode {
    /// Error inesperado.
    Internal,
    /// No se pudo crear o mostrar una ventana.
    WindowFailed,
    /// No se pudo consultar el servidor de actualizaciones.
    UpdateCheckFailed,
    /// No hay ninguna actualización para instalar.
    UpdateUnavailable,
    /// La descarga o la instalación de la actualización falló.
    UpdateInstallFailed,
    /// El motor no está disponible.
    EngineUnavailable,
    /// El monitor no lista ese modo.
    ModeNotSupported,
    /// Hay otro cambio de resolución en curso o esperando confirmación.
    ModeBusy,
    /// Windows no pudo aplicar o revertir el modo.
    ModeFailed,
    /// El cambio ya se confirmó, se revirtió o venció.
    ModeExpired,
    /// El atajo solo alterna a modos que ya conservaste una vez.
    ModeNotConfirmed,
    /// No se pudo cambiar el HDR.
    HdrFailed,
    /// La pantalla ya no está conectada.
    DisplayGone,
    /// El atajo no es válido.
    HotkeyInvalid,
    /// Otro programa ya usa ese atajo.
    HotkeyTaken,
    /// No se pudieron guardar los ajustes.
    SettingsFailed,
}

/// Error que devuelven todos los comandos de Tauri.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, thiserror::Error)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
#[error("{code:?}: {message}")]
pub struct IpcError {
    /// Código estable (para traducir).
    pub code: ErrorCode,
    /// Detalle técnico en inglés, solo para logs y reportes.
    pub message: String,
}

impl IpcError {
    /// Crea un error con su código y un detalle técnico.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_serializes_in_camel_case() {
        let value = serde_json::to_value(Bootstrap {
            app_version: "0.0.1".into(),
            system_language: Some("es-AR".into()),
            autostarted: false,
            displays: Vec::new(),
            requested: Requested::default(),
            status: EngineStatus::default(),
            notices: vec![Notice::SafeMode],
        })
        .expect("serializable");
        assert_eq!(value["appVersion"], "0.0.1");
        assert_eq!(value["systemLanguage"], "es-AR");
        assert_eq!(value["autostarted"], false);
        assert_eq!(value["requested"]["colorEnabled"], true);
        assert_eq!(value["notices"][0], "safeMode");
    }

    #[test]
    fn error_code_is_stable_text() {
        let value = serde_json::to_value(ErrorCode::UpdateUnavailable).expect("serializable");
        assert_eq!(value, "updateUnavailable");
    }
}
