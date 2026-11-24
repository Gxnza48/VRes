//! VRes: aplicación de escritorio (Tauri 2).
//!
//! Este crate solo conecta piezas: la interfaz (WebView) ⇄ los comandos ⇄ el
//! motor. La lógica vive en `vres-core` (pura), el acceso a Windows en
//! `vres-win` y el motor en `vres-engine`.
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
mod engine_bridge;
mod i18n;
mod lifecycle;
mod logging;
mod settings;
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
    // Ante un error interno, la pantalla vuelve a la normalidad antes de
    // terminar (la matriz de color la quita Windows sola con el proceso).
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        vres_engine::emergency_restore();
        default_hook(info);
    }));

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
            commands::app::app_open_main,
            commands::app::quick_toggle,
            commands::app::quick_close,
            commands::engine::look_get,
            commands::update::update_check,
            commands::update::update_install,
            commands::engine::displays_list,
            commands::engine::displays_identify,
            commands::engine::displays_set_targets,
            commands::engine::look_preview,
            commands::engine::look_commit,
            commands::engine::look_reset,
            commands::engine::look_set_enabled,
            commands::engine::look_compare,
            commands::engine::engine_set_paused,
            commands::engine::engine_emergency_reset,
            commands::engine::engine_status,
            commands::settings::settings_get_autostart,
            commands::settings::settings_set_autostart,
            commands::settings::settings_get,
            commands::settings::settings_update,
            commands::settings::hotkeys_check,
            commands::settings::hotkeys_suspend,
            commands::resolution::resolution_get_state,
            commands::resolution::resolution_apply,
            commands::resolution::resolution_confirm,
            commands::resolution::resolution_revert,
            commands::resolution::resolution_restore,
            commands::resolution::resolution_toggle,
            commands::resolution::hdr_set,
            commands::crosshair::crosshair_render,
            commands::crosshair::crosshair_presets,
            commands::crosshair::crosshair_preview,
            commands::share::share_encode_look,
            commands::share::share_encode_crosshair,
            commands::share::share_encode_resolution,
            commands::share::share_decode,
            commands::share::looks_presets,
            commands::share::games_list,
            commands::profiles::profile_active,
            commands::profiles::games_running_candidates,
            commands::enhancers::enhancers_state,
            commands::enhancers::enhancers_key_supported,
            commands::customize::customize_background_set,
            commands::customize::customize_background_get,
            commands::customize::customize_background_clear,
            commands::diag::diag_report,
            commands::diag::settings_reset,
        ])
        .build(tauri::generate_context!())
        .expect("VRes no pudo iniciar");

    app.run(lifecycle::on_run_event);
}
