//! Registro de eventos en archivo.
//!
//! `%LOCALAPPDATA%\io.github.gxnza48.vres\logs\vres.log`. El log de la sesión
//! anterior se conserva como `vres.prev.log` para poder diagnosticar un cierre
//! inesperado. Nunca sale de la PC.

use std::fs;
use std::sync::Mutex;

use tauri::{AppHandle, Manager};

/// Inicia el registro. Si falla, VRes sigue funcionando sin log.
pub fn init(app: &AppHandle) {
    let Ok(dir) = app.path().app_log_dir() else {
        return;
    };
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let current = dir.join("vres.log");
    let previous = dir.join("vres.prev.log");
    if current.exists() {
        let _ = fs::rename(&current, &previous);
    }
    let Ok(file) = fs::File::create(&current) else {
        return;
    };
    let _ = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_target(false)
        .with_max_level(tracing::Level::INFO)
        .with_writer(Mutex::new(file))
        .try_init();
}
