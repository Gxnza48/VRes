//! Ventanas de la interfaz.
//!
//! La ventana principal no existe hasta que hace falta (`"create": false` en
//! `tauri.conf.json`) y se **destruye** al cerrarla, para liberar la memoria de
//! WebView2 mientras se juega. VRes sigue vivo en la bandeja.

use std::time::Duration;

use tauri::{AppHandle, Manager, WebviewWindow, WebviewWindowBuilder};

/// Etiqueta de la ventana principal.
pub const MAIN: &str = "main";

/// Si la interfaz no avisa que está lista (`app_ui_ready`), la ventana se
/// muestra igual después de este tiempo.
const SHOW_FALLBACK: Duration = Duration::from_millis(1500);

/// Abre la ventana principal, o la enfoca si ya existe.
///
/// Solo se puede llamar desde `setup` o desde una tarea async: crear ventanas
/// desde un handler síncrono provoca un deadlock en Windows. Para todo lo demás
/// está [`open_main_async`].
pub fn open_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN) {
        focus(&window);
        return;
    }

    let Some(config) = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == MAIN)
        .cloned()
    else {
        tracing::error!("falta la ventana `main` en tauri.conf.json");
        return;
    };

    match WebviewWindowBuilder::from_config(app, &config).and_then(|builder| builder.build()) {
        Ok(window) => {
            // Se crea oculta para evitar el destello blanco: la interfaz la
            // muestra cuando terminó de pintar. Si no avisa, se muestra igual.
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(SHOW_FALLBACK).await;
                if !window.is_visible().unwrap_or(true) {
                    tracing::warn!("la interfaz no avisó que estaba lista; se muestra igual");
                    focus(&window);
                }
            });
        }
        Err(error) => tracing::error!(%error, "no se pudo crear la ventana principal"),
    }
}

/// Igual que [`open_main`], pero seguro desde cualquier handler (bandeja,
/// segunda instancia, atajos): crea la ventana en una tarea async.
pub fn open_main_async(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move { open_main(&app) });
}

/// Muestra, restaura y enfoca una ventana.
pub fn focus(window: &WebviewWindow) {
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}
