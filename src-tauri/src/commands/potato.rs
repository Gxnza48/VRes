//! Comandos de los gráficos potato (perfil del juego en el driver de NVIDIA).

use std::time::Duration;

use vres_core::ipc::{ErrorCode, IpcError};
use vres_core::potato::{self, PotatoGame, PotatoLevel, PotatoStatus};

fn level_arg(level: PotatoLevel) -> &'static str {
    match level {
        PotatoLevel::Off => "off",
        PotatoLevel::Low => "low",
        PotatoLevel::Potato => "potato",
    }
}

fn parse_level(text: &str) -> Option<PotatoLevel> {
    match text {
        "off" => Some(PotatoLevel::Off),
        "low" => Some(PotatoLevel::Low),
        "potato" => Some(PotatoLevel::Potato),
        _ => None,
    }
}

fn game(id: &str) -> Option<PotatoGame> {
    potato::games().into_iter().find(|game| game.id == id)
}

#[cfg(windows)]
fn all_status() -> Vec<PotatoStatus> {
    potato::games()
        .iter()
        .map(vres_win::nvdrs::potato_status)
        .collect()
}

#[cfg(not(windows))]
fn all_status() -> Vec<PotatoStatus> {
    Vec::new()
}

/// `vres.exe --potato JUEGO NIVEL` (ya como administrador).
pub fn apply_from_flag(game_id: &str, level: &str) -> i32 {
    let (Some(game), Some(level)) = (game(game_id), parse_level(level)) else {
        return 2;
    };
    #[cfg(windows)]
    {
        match vres_win::nvdrs::potato_apply(&game, level) {
            Ok(()) => 0,
            Err(error) => {
                tracing::warn!("potato apply failed: {error}");
                2
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (game, level);
        2
    }
}

/// Estado del potato de cada juego (solo lee el driver).
#[tauri::command]
pub async fn potato_status() -> Result<Vec<PotatoStatus>, IpcError> {
    tauri::async_runtime::spawn_blocking(all_status)
        .await
        .map_err(|error| IpcError::new(ErrorCode::Internal, error.to_string()))
}

/// Pone un nivel en un juego. Guardar en el driver exige administrador: si
/// hace falta, VRes se relanza elevado solo para este paso (aviso de UAC).
#[tauri::command]
pub async fn potato_apply(game: String, level: PotatoLevel) -> Result<Vec<PotatoStatus>, IpcError> {
    tauri::async_runtime::spawn_blocking(move || apply_blocking(&game, level))
        .await
        .map_err(|error| IpcError::new(ErrorCode::Internal, error.to_string()))?
}

#[cfg(windows)]
fn apply_blocking(game_id: &str, level: PotatoLevel) -> Result<Vec<PotatoStatus>, IpcError> {
    use vres_win::{nvdrs, process};

    let Some(target) = game(game_id) else {
        return Err(IpcError::new(ErrorCode::PotatoUnavailable, "unknown game"));
    };
    match nvdrs::potato_apply(&target, level) {
        Ok(()) => {}
        Err(error) if nvdrs::needs_admin(&error) => {
            let args = format!("--potato {} {}", target.id, level_arg(level));
            match process::run_self_elevated(&args, Duration::from_secs(60)) {
                Ok(Some(0)) => {}
                Ok(None) => return Err(IpcError::new(ErrorCode::PotatoCancelled, "uac declined")),
                Ok(Some(code)) => {
                    return Err(IpcError::new(
                        ErrorCode::PotatoFailed,
                        format!("exit code {code}"),
                    ));
                }
                Err(error) => {
                    return Err(IpcError::new(ErrorCode::PotatoFailed, error.to_string()));
                }
            }
        }
        Err(error) => {
            let code = if nvdrs::available() {
                ErrorCode::PotatoFailed
            } else {
                ErrorCode::PotatoUnavailable
            };
            return Err(IpcError::new(code, error.to_string()));
        }
    }
    let status = all_status();
    let applied = status
        .iter()
        .any(|status| status.game.id == target.id && status.level == level);
    if applied {
        Ok(status)
    } else {
        Err(IpcError::new(
            ErrorCode::PotatoFailed,
            "read back a different level",
        ))
    }
}

#[cfg(not(windows))]
fn apply_blocking(_game: &str, _level: PotatoLevel) -> Result<Vec<PotatoStatus>, IpcError> {
    Err(IpcError::new(ErrorCode::PotatoUnavailable, "windows only"))
}
