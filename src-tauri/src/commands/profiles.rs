//! Comandos de los perfiles por juego.

use tauri::State;
use vres_core::ipc::{IpcError, RunningApp};

use crate::engine_bridge::EngineState;

/// Perfil de juego en uso (`null`: el escritorio).
#[tauri::command]
pub async fn profile_active(engine: State<'_, EngineState>) -> Result<Option<String>, IpcError> {
    Ok(engine.active_profile())
}

/// Programas abiertos con ventana, para elegir un juego propio.
#[tauri::command]
pub async fn games_running_candidates() -> Result<Vec<RunningApp>, IpcError> {
    Ok(vres_win::process::windowed_apps()
        .into_iter()
        .map(|app| RunningApp {
            executable: app.exe,
            title: app.title,
        })
        .collect())
}
