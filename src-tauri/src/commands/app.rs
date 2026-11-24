//! Comandos generales de la aplicación.

use tauri::{AppHandle, State, WebviewWindow};
use vres_core::ipc::{Bootstrap, IpcError};

use crate::LaunchInfo;
use crate::app_windows;

/// Datos iniciales para la interfaz.
#[tauri::command]
pub async fn app_bootstrap(launch: State<'_, LaunchInfo>) -> Result<Bootstrap, IpcError> {
    Ok(Bootstrap {
        app_version: vres_core::APP_VERSION.to_owned(),
        system_language: sys_locale::get_locale(),
        autostarted: launch.autostarted,
    })
}

/// La interfaz terminó de pintar: se muestra su ventana (sin destello blanco).
#[tauri::command]
pub async fn app_ui_ready(window: WebviewWindow) -> Result<(), IpcError> {
    app_windows::focus(&window);
    Ok(())
}

/// Cierra VRes por completo (no solo la ventana).
#[tauri::command]
pub async fn app_quit(app: AppHandle) -> Result<(), IpcError> {
    app.exit(0);
    Ok(())
}
