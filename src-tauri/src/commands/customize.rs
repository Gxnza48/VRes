//! Comandos de Personalizar: la imagen o GIF de fondo. Se guarda en
//! `%APPDATA%\io.github.gxnza48.vres\media\background.bin` y nunca sale de
//! la PC.

use std::path::PathBuf;

use tauri::ipc::{InvokeBody, Request, Response};
use tauri::{AppHandle, Emitter, Manager};
use vres_core::appearance::{BackgroundImage, MAX_BACKGROUND_BYTES, image_mime};
use vres_core::ipc::{ErrorCode, IpcError};
use vres_core::settings::Settings;
use vres_core::store;

use crate::engine_bridge::EVENT_SETTINGS;
use crate::settings::SettingsState;

fn background_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|dir| {
        PathBuf::from(dir)
            .join(vres_core::APP_IDENTIFIER)
            .join("media")
            .join("background.bin")
    })
}

fn invalid(detail: &str) -> IpcError {
    IpcError::new(ErrorCode::BackgroundInvalid, detail)
}

fn internal(error: impl ToString) -> IpcError {
    IpcError::new(ErrorCode::Internal, error.to_string())
}

/// Guarda los ajustes con la imagen nueva (o sin imagen) y avisa.
fn save_background(app: &AppHandle, image: Option<BackgroundImage>) -> Result<Settings, IpcError> {
    let state = app
        .try_state::<SettingsState>()
        .ok_or_else(|| internal("settings state missing"))?;
    let mut settings = state.get();
    settings.appearance.background = image;
    state
        .set(settings.clone())
        .map_err(|error| IpcError::new(ErrorCode::SettingsFailed, error))?;
    let _ = app.emit(EVENT_SETTINGS, &settings);
    Ok(settings)
}

/// Usa una imagen o un GIF como fondo (los bytes llegan tal cual, sin JSON).
#[tauri::command]
pub async fn customize_background_set(
    app: AppHandle,
    request: Request<'_>,
) -> Result<Settings, IpcError> {
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err(invalid("expected the image bytes"));
    };
    if bytes.len() > MAX_BACKGROUND_BYTES {
        return Err(invalid("the image is too large"));
    }
    let mime = image_mime(bytes).ok_or_else(|| invalid("not a PNG, JPEG, GIF or WebP image"))?;
    let bytes = bytes.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let path = background_path().ok_or_else(|| internal("no settings folder"))?;
        store::write_bytes_atomic(&path, &bytes).map_err(internal)?;
        let revision = app
            .try_state::<SettingsState>()
            .and_then(|state| state.get().appearance.background)
            .map_or(1, |image| image.revision.wrapping_add(1));
        save_background(
            &app,
            Some(BackgroundImage {
                mime: mime.to_owned(),
                revision,
            }),
        )
    })
    .await
    .map_err(internal)?
}

/// La imagen de fondo, en bytes (vacío si no hay).
#[tauri::command]
pub async fn customize_background_get() -> Result<Response, IpcError> {
    let bytes = tauri::async_runtime::spawn_blocking(|| {
        background_path()
            .and_then(|path| std::fs::read(path).ok())
            .unwrap_or_default()
    })
    .await
    .map_err(internal)?;
    Ok(Response::new(bytes))
}

/// Quita el fondo.
#[tauri::command]
pub async fn customize_background_clear(app: AppHandle) -> Result<Settings, IpcError> {
    tauri::async_runtime::spawn_blocking(move || {
        if let Some(path) = background_path() {
            let _ = std::fs::remove_file(path);
        }
        save_background(&app, None)
    })
    .await
    .map_err(internal)?
}
