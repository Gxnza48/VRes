//! Ajustes de la app.

use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;
use vres_core::ipc::{ErrorCode, IpcError};

fn failed(error: impl ToString) -> IpcError {
    IpcError::new(ErrorCode::Internal, error.to_string())
}

/// `true` si VRes arranca con Windows (directo a la bandeja).
#[tauri::command]
pub async fn settings_get_autostart(app: AppHandle) -> Result<bool, IpcError> {
    app.autolaunch().is_enabled().map_err(failed)
}

/// Activa o desactiva el arranque con Windows. Devuelve el estado final.
#[tauri::command]
pub async fn settings_set_autostart(app: AppHandle, enabled: bool) -> Result<bool, IpcError> {
    let autolaunch = app.autolaunch();
    if enabled {
        autolaunch.enable().map_err(failed)?;
    } else {
        autolaunch.disable().map_err(failed)?;
    }
    autolaunch.is_enabled().map_err(failed)
}
