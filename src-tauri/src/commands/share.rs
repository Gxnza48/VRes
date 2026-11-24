//! Códigos para compartir y looks incluidos.

use vres_core::crosshair::Crosshair;
use vres_core::ipc::{GameInfo, IpcError, ShareDecoded};
use vres_core::looks::{self, LookPreset};
use vres_core::model::look::Look;
use vres_core::model::resolution::ModeRequest;
use vres_core::share::{self, ShareKind};

/// Código de un look.
#[tauri::command]
pub async fn share_encode_look(look: Look) -> Result<String, IpcError> {
    Ok(share::encode_look(&look))
}

/// Código de un crosshair (`None` si es de imagen: no entra en un código).
#[tauri::command]
pub async fn share_encode_crosshair(crosshair: Crosshair) -> Result<Option<String>, IpcError> {
    Ok(share::encode_crosshair(&crosshair).ok())
}

/// Código de una resolución.
#[tauri::command]
pub async fn share_encode_resolution(mode: ModeRequest) -> Result<String, IpcError> {
    Ok(share::encode_resolution(&mode))
}

/// Lee cualquier código de VRes. Nunca falla: el motivo viaja en `error`.
#[tauri::command]
pub async fn share_decode(code: String) -> Result<ShareDecoded, IpcError> {
    let mut decoded = ShareDecoded {
        kind: None,
        look: None,
        crosshair: None,
        resolution: None,
        clamped: false,
        error: None,
    };
    match share::kind_of(&code) {
        Err(error) => decoded.error = Some(error),
        Ok(kind) => {
            decoded.kind = Some(kind);
            let result = match kind {
                ShareKind::Look => share::decode_look(&code).map(|d| {
                    decoded.clamped = d.clamped;
                    decoded.look = Some(d.value);
                }),
                ShareKind::Crosshair => share::decode_crosshair(&code).map(|d| {
                    decoded.clamped = d.clamped;
                    decoded.crosshair = Some(d.value);
                }),
                ShareKind::Resolution => share::decode_resolution(&code).map(|d| {
                    decoded.resolution = Some(d.value);
                }),
            };
            if let Err(error) = result {
                decoded.error = Some(error);
            }
        }
    }
    Ok(decoded)
}

/// Looks incluidos en VRes.
#[tauri::command]
pub async fn looks_presets() -> Result<Vec<LookPreset>, IpcError> {
    Ok(looks::presets())
}

/// Juegos conocidos.
#[tauri::command]
pub async fn games_list() -> Result<Vec<GameInfo>, IpcError> {
    Ok(looks::GAMES
        .iter()
        .map(|(id, name)| GameInfo {
            id: (*id).to_owned(),
            name: (*name).to_owned(),
        })
        .collect())
}
