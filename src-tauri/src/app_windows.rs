//! Ventanas de la interfaz.
//!
//! La ventana principal no existe hasta que hace falta (`"create": false` en
//! `tauri.conf.json`) y se **destruye** al cerrarla, para liberar la memoria de
//! WebView2 mientras se juega. VRes sigue vivo en la bandeja.

use std::time::Duration;

use tauri::{AppHandle, Manager, WebviewWindow, WebviewWindowBuilder};

/// Etiqueta de la ventana principal.
pub const MAIN: &str = "main";
/// Etiqueta del popup rápido.
pub const QUICK: &str = "quick";

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

/// Abre el popup rápido junto a la bandeja, o lo cierra si ya está abierto.
/// Como la ventana principal, se crea a demanda y se destruye al cerrarla.
/// Solo desde una tarea async (ver [`toggle_quick_async`]).
pub fn toggle_quick(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(QUICK) {
        let _ = window.destroy();
        return;
    }
    let Some(config) = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == QUICK)
        .cloned()
    else {
        tracing::error!("falta la ventana `quick` en tauri.conf.json");
        return;
    };
    match WebviewWindowBuilder::from_config(app, &config).and_then(|builder| builder.build()) {
        // Se muestra cuando la interfaz avisa que pintó (`app_ui_ready`).
        Ok(window) => place_near_tray(&window),
        Err(error) => tracing::error!(%error, "no se pudo crear el popup rápido"),
    }
}

/// Igual que [`toggle_quick`], seguro desde cualquier handler.
pub fn toggle_quick_async(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move { toggle_quick(&app) });
}

/// Cierra el popup rápido, si está abierto.
pub fn close_quick(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(QUICK) {
        let _ = window.destroy();
    }
}

/// Esquina inferior derecha del área de trabajo del monitor principal (al
/// lado de la bandeja).
fn place_near_tray(window: &WebviewWindow) {
    let Ok(Some(monitor)) = window.primary_monitor() else {
        return;
    };
    let area = monitor.work_area();
    let Ok(size) = window.outer_size() else {
        return;
    };
    let margin = (12.0 * monitor.scale_factor()).round() as i32;
    let x = area.position.x + area.size.width as i32 - size.width as i32 - margin;
    let y = area.position.y + area.size.height as i32 - size.height as i32 - margin;
    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
}

/// Muestra, restaura y enfoca una ventana.
pub fn focus(window: &WebviewWindow) {
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}
