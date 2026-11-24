//! Comandos de Dark Boost y Zoom (los ajustes van por `settings_update`).

use tauri::State;
use vres_core::ipc::{EnhancerState, IpcError};

use crate::engine_bridge::EngineState;

/// Si una tecla física (`KeyboardEvent.code`) sirve para Dark Boost o Zoom.
#[tauri::command]
pub async fn enhancers_key_supported(code: String) -> Result<bool, IpcError> {
    Ok(vres_core::keys::hold_key_spec(&code).is_some())
}

/// Si Dark Boost y Zoom están escuchando su tecla o activos.
#[tauri::command]
pub async fn enhancers_state(engine: State<'_, EngineState>) -> Result<EnhancerState, IpcError> {
    Ok(engine.enhancer_state())
}
