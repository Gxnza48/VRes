//! Comandos del motor: pantallas, look y estado.

use tauri::{AppHandle, State};
use vres_core::ipc::{ErrorCode, IpcError};
use vres_core::model::display::DisplayInfo;
use vres_core::model::look::Look;
use vres_core::planner::Requested;
use vres_core::status::EngineStatus;
use vres_engine::{ApplyMode, Command};

use crate::engine_bridge::EngineState;

fn ensure(engine: &EngineState) -> Result<(), IpcError> {
    if engine.available() {
        Ok(())
    } else {
        Err(IpcError::new(
            ErrorCode::EngineUnavailable,
            "the display engine is not running",
        ))
    }
}

/// Pantallas activas.
#[tauri::command]
pub async fn displays_list(engine: State<'_, EngineState>) -> Result<Vec<DisplayInfo>, IpcError> {
    Ok(engine.displays())
}

/// Muestra el número de cada pantalla durante unos segundos.
#[tauri::command]
pub async fn displays_identify(engine: State<'_, EngineState>) -> Result<(), IpcError> {
    ensure(&engine)?;
    engine.send(Command::Identify);
    Ok(())
}

/// Elige a qué pantallas se aplica el look (`None` = todas).
#[tauri::command]
pub async fn displays_set_targets(
    engine: State<'_, EngineState>,
    targets: Option<Vec<String>>,
) -> Result<Requested, IpcError> {
    ensure(&engine)?;
    Ok(engine.update(ApplyMode::Commit, |requested| {
        requested.targets = targets.map(|ids| ids.into_iter().collect());
    }))
}

/// Vista previa mientras se arrastra un control (no se guarda).
#[tauri::command]
pub async fn look_preview(engine: State<'_, EngineState>, look: Look) -> Result<(), IpcError> {
    ensure(&engine)?;
    engine.update(ApplyMode::Preview, |requested| requested.look = look);
    Ok(())
}

/// Aplica y guarda un look.
#[tauri::command]
pub async fn look_commit(
    engine: State<'_, EngineState>,
    look: Look,
) -> Result<Requested, IpcError> {
    ensure(&engine)?;
    Ok(engine.update(ApplyMode::Commit, |requested| requested.look = look))
}

/// Vuelve el look a los valores neutros.
#[tauri::command]
pub async fn look_reset(engine: State<'_, EngineState>) -> Result<Requested, IpcError> {
    ensure(&engine)?;
    Ok(engine.update(ApplyMode::Commit, |requested| {
        requested.look = Look::default();
    }))
}

/// Enciende o apaga el color.
#[tauri::command]
pub async fn look_set_enabled(
    engine: State<'_, EngineState>,
    enabled: bool,
) -> Result<Requested, IpcError> {
    ensure(&engine)?;
    Ok(engine.update(ApplyMode::Commit, |requested| {
        requested.color_enabled = enabled;
    }))
}

/// "Comparar": mientras está activo se ve la imagen original.
#[tauri::command]
pub async fn look_compare(engine: State<'_, EngineState>, active: bool) -> Result<(), IpcError> {
    ensure(&engine)?;
    engine.update(ApplyMode::Preview, |requested| requested.compare = active);
    Ok(())
}

/// Pausa o reanuda todo.
#[tauri::command]
pub async fn engine_set_paused(
    app: AppHandle,
    engine: State<'_, EngineState>,
    paused: bool,
) -> Result<Requested, IpcError> {
    ensure(&engine)?;
    let requested = engine.update(ApplyMode::Commit, |requested| requested.paused = paused);
    crate::tray::set_paused(&app, paused);
    Ok(requested)
}

/// Vuelve la pantalla a la normalidad y apaga el color.
#[tauri::command]
pub async fn engine_emergency_reset(engine: State<'_, EngineState>) -> Result<(), IpcError> {
    ensure(&engine)?;
    engine.send(Command::EmergencyReset);
    Ok(())
}

/// Estado de cada capa.
#[tauri::command]
pub async fn engine_status(engine: State<'_, EngineState>) -> Result<EngineStatus, IpcError> {
    Ok(engine.status())
}
