//! Actualizaciones desde GitHub Releases.
//!
//! Es la única conexión saliente de VRes, y el usuario la puede desactivar.
//! Cada instalador está firmado: el updater rechaza cualquier archivo cuya
//! firma no coincida con la clave pública de `tauri.conf.json`.

use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;
use vres_core::ipc::{ErrorCode, IpcError, UpdateInfo, UpdateProgress};

use crate::lifecycle;

fn check_failed(error: impl ToString) -> IpcError {
    IpcError::new(ErrorCode::UpdateCheckFailed, error.to_string())
}

fn install_failed(error: impl ToString) -> IpcError {
    IpcError::new(ErrorCode::UpdateInstallFailed, error.to_string())
}

/// Consulta si hay una versión nueva.
#[tauri::command]
pub async fn update_check(app: AppHandle) -> Result<Option<UpdateInfo>, IpcError> {
    let updater = app.updater().map_err(check_failed)?;
    let update = updater.check().await.map_err(check_failed)?;
    Ok(update.map(|update| UpdateInfo {
        version: update.version.clone(),
        current_version: update.current_version.clone(),
        notes: update.body.clone(),
        date: update
            .raw_json
            .get("pub_date")
            .and_then(|value| value.as_str())
            .map(str::to_owned),
    }))
}

/// Descarga e instala la versión nueva.
///
/// En Windows el instalador cierra VRes sin pasar por `RunEvent::Exit`, así
/// que primero se apaga todo con el hook `on_before_exit`.
#[tauri::command]
pub async fn update_install(app: AppHandle) -> Result<(), IpcError> {
    let exit_app = app.clone();
    let updater = app
        .updater_builder()
        .on_before_exit(move || lifecycle::shutdown(&exit_app))
        .build()
        .map_err(install_failed)?;

    let Some(update) = updater.check().await.map_err(check_failed)? else {
        return Err(IpcError::new(
            ErrorCode::UpdateUnavailable,
            "no update available",
        ));
    };

    let progress_app = app.clone();
    let mut downloaded: u64 = 0;
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let _ = progress_app.emit("update:progress", UpdateProgress { downloaded, total });
            },
            || {},
        )
        .await
        .map_err(install_failed)?;

    // En Windows no se llega acá: el instalador cierra VRes y lo vuelve a abrir.
    app.restart();
}
