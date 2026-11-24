//! Comandos de resolución y HDR.
//!
//! Cambiar de modo puede tardar unos segundos: la espera corre fuera del
//! runtime async (`spawn_blocking`) para no trabar otros comandos.

use std::time::Duration;

use tauri::{AppHandle, Manager, State};
use vres_core::ipc::{ErrorCode, IpcError};
use vres_core::model::resolution::{ModeRequest, PendingMode, ResolutionState};
use vres_engine::Command;

use crate::engine_bridge::EngineState;

/// Tope para un cambio de modo o de HDR.
const MODE_TIMEOUT: Duration = Duration::from_secs(15);
/// Tope para confirmar (no toca la pantalla).
const CONFIRM_TIMEOUT: Duration = Duration::from_secs(3);

async fn blocking<T: Send + 'static>(
    app: AppHandle,
    work: impl FnOnce(&EngineState) -> Result<T, IpcError> + Send + 'static,
) -> Result<T, IpcError> {
    tauri::async_runtime::spawn_blocking(move || {
        let engine = app
            .try_state::<EngineState>()
            .ok_or_else(|| IpcError::new(ErrorCode::EngineUnavailable, "engine state missing"))?;
        if !engine.available() {
            return Err(IpcError::new(
                ErrorCode::EngineUnavailable,
                "the display engine is not running",
            ));
        }
        work(&engine)
    })
    .await
    .map_err(|error| IpcError::new(ErrorCode::Internal, error.to_string()))?
}

/// Modos de cada pantalla, presets y el cambio pendiente.
#[tauri::command]
pub async fn resolution_get_state(
    engine: State<'_, EngineState>,
) -> Result<ResolutionState, IpcError> {
    Ok(engine.resolution())
}

/// Prueba un modo. Queda pendiente: si no se confirma a tiempo, vuelve solo.
#[tauri::command]
pub async fn resolution_apply(
    app: AppHandle,
    display_id: String,
    request: ModeRequest,
) -> Result<PendingMode, IpcError> {
    blocking(app, move |engine| {
        engine.request(
            |reply| Command::ModeApply {
                display_id,
                request,
                reply,
            },
            MODE_TIMEOUT,
        )
    })
    .await
}

/// Conserva el modo pendiente.
#[tauri::command]
pub async fn resolution_confirm(app: AppHandle, token: String) -> Result<(), IpcError> {
    blocking(app, move |engine| {
        engine.request(
            |reply| Command::ModeConfirm { token, reply },
            CONFIRM_TIMEOUT,
        )
    })
    .await
}

/// Vuelve atrás el modo pendiente.
#[tauri::command]
pub async fn resolution_revert(app: AppHandle, token: String) -> Result<(), IpcError> {
    blocking(app, move |engine| {
        engine.request(|reply| Command::ModeRevert { token, reply }, MODE_TIMEOUT)
    })
    .await
}

/// Vuelve a la resolución de Windows en una pantalla.
#[tauri::command]
pub async fn resolution_restore(app: AppHandle, display_id: String) -> Result<(), IpcError> {
    blocking(app, move |engine| {
        engine.request(
            |reply| Command::ModeRestore { display_id, reply },
            MODE_TIMEOUT,
        )
    })
    .await
}

/// Lo mismo que el atajo: alterna entre la de Windows y la estirada.
#[tauri::command]
pub async fn resolution_toggle(engine: State<'_, EngineState>) -> Result<(), IpcError> {
    if engine.send(Command::ModeToggle) {
        Ok(())
    } else {
        Err(IpcError::new(
            ErrorCode::EngineUnavailable,
            "the display engine is not running",
        ))
    }
}

/// Prende o apaga HDR en una pantalla (se restaura al salir de VRes).
#[tauri::command]
pub async fn hdr_set(app: AppHandle, display_id: String, enabled: bool) -> Result<(), IpcError> {
    blocking(app, move |engine| {
        engine.request(
            |reply| Command::HdrSet {
                display_id,
                enabled,
                reply,
            },
            MODE_TIMEOUT,
        )
    })
    .await
}
