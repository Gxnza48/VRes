//! Comandos del crosshair: vista previa dibujada por el mismo renderer que la
//! ventana sobre el juego, presets propios y vista previa en vivo.

use tauri::State;
use tauri::ipc::Response;
use vres_core::crosshair::{self, Crosshair, CrosshairPreset};
use vres_core::ipc::{ErrorCode, IpcError};
use vres_engine::Command;

use crate::engine_bridge::EngineState;

/// Dibuja un crosshair. Responde bytes: lado (`u32` LE), centro (`u32` LE) y
/// RGBA sin premultiplicar (lo que espera un `<canvas>`).
#[tauri::command]
pub async fn crosshair_render(crosshair: Crosshair) -> Result<Response, IpcError> {
    let rendered = crosshair::render(&crosshair)
        .map_err(|error| IpcError::new(ErrorCode::CrosshairInvalid, error.to_string()))?;
    let rgba = rendered.to_straight_rgba();
    let mut bytes = Vec::with_capacity(8 + rgba.len());
    bytes.extend_from_slice(&rendered.size.to_le_bytes());
    bytes.extend_from_slice(&rendered.center.to_le_bytes());
    bytes.extend_from_slice(&rgba);
    Ok(Response::new(bytes))
}

/// Presets propios de VRes.
#[tauri::command]
pub async fn crosshair_presets() -> Result<Vec<CrosshairPreset>, IpcError> {
    Ok(crosshair::presets())
}

/// Muestra este crosshair en pantalla mientras se edita (sin guardar).
/// `None` vuelve al guardado.
#[tauri::command]
pub async fn crosshair_preview(
    engine: State<'_, EngineState>,
    crosshair: Option<Crosshair>,
) -> Result<(), IpcError> {
    engine.send(Command::CrosshairPreview(crosshair.map(Box::new)));
    Ok(())
}
