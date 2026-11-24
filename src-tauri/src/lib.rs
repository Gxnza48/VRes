//! VRes: aplicación de escritorio (Tauri 2).
//!
//! Este crate solo conecta piezas: la interfaz (WebView) ⇄ los comandos ⇄ el
//! motor. La lógica vive en `vres-core` (pura) y el acceso a Windows en
//! `vres-win`.
//!
//! Reglas de Tauri que hay que respetar (verificadas en su código fuente):
//! - todos los comandos son `async` (los síncronos corren en el hilo principal);
//! - las ventanas se crean desde `setup` o desde una tarea async, nunca desde un
//!   handler síncrono (deadlock en Windows);
//! - `single-instance` es el primer plugin;
//! - la limpieza va en `RunEvent::Exit` (y además en el hook del updater, que en
//!   Windows termina el proceso sin pasar por ahí).
#![forbid(unsafe_code)]

mod app_windows;
mod commands;
mod early;
mod i18n;
mod lifecycle;
mod logging;
mod tray;

use tauri::DeviceEventFilter;
use tauri_plugin_autostart::MacosLauncher;

pub use early::handle_early_flags;

/// Argumento con el que Windows lanza VRes al iniciar sesión.
/// En ese caso VRes arranca en la bandeja, sin abrir la ventana.
pub const AUTOSTART_FLAG: &str = "--autostart";

/// Información del arranque, consultable desde los comandos.
#[derive(Debug, Clone, Copy)]
pub struct LaunchInfo {
    /// `true` si VRes arrancó solo con Windows.
    pub autostarted: bool,
}

/// Arranca la aplicación. No retorna: al salir, Tauri termina el proceso.
pub fn run() {
    let app = tauri::Builder::default()
        // Debe ser el primer plugin: una segunda instancia le pasa sus
        // argumentos a la primera y termina.
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            lifecycle::on_second_instance(app, &args);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![AUTOSTART_FLAG]),
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        // VRes no usa Raw Input de la ventana de Tauri: así el motor puede
        // registrarlo sin pisarse y no se procesa cada movimiento del mouse.
        .device_event_filter(DeviceEventFilter::Always)
        .setup(|app| lifecycle::setup(app))
        .invoke_handler(tauri::generate_handler![
            commands::app::app_bootstrap,
            commands::app::app_ui_ready,
            commands::app::app_quit,
            commands::update::update_check,
            commands::update::update_install,
        ])
        .build(tauri::generate_context!())
        .expect("VRes no pudo iniciar");

    app.run(lifecycle::on_run_event);
}
