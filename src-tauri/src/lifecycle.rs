//! Ciclo de vida: arranque, segunda instancia, cierre de ventanas y salida.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{App, AppHandle, Manager, RunEvent};
use vres_engine::Command;

use crate::engine_bridge::EngineState;
use crate::settings::SettingsState;
use crate::{AUTOSTART_FLAG, LaunchInfo, app_windows, logging, tray};

/// Inicialización, antes de que corra el bucle de eventos.
pub fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();
    logging::init(&handle);
    tracing::info!(version = vres_core::APP_VERSION, "VRes iniciando");

    let autostarted = std::env::args().any(|arg| arg == AUTOSTART_FLAG);
    app.manage(LaunchInfo { autostarted });

    // El motor arranca antes que la bandeja y la ventana: restaura lo que haya
    // quedado de una sesión anterior y aplica la sesión guardada.
    // Los ajustes primero: el motor arranca con el atajo y sus preferencias.
    let settings = SettingsState::load();
    let engine = EngineState::start(&handle, &settings.get());
    app.manage(settings);
    let paused = engine.requested().paused;
    app.manage(engine);

    tray::create(&handle, paused)?;

    if !autostarted {
        // En `setup` todavía no corre el bucle de eventos: crear la ventana
        // acá es seguro (no es un handler síncrono).
        app_windows::open_main(&handle);
    }
    Ok(())
}

/// Alguien abrió VRes otra vez: se enfoca la ventana existente.
pub fn on_second_instance(app: &AppHandle, args: &[String]) {
    // Normalmente `--shutdown` y `--reset` se atienden en `main()` antes de
    // arrancar Tauri; esto cubre la carrera en la que igual llegan acá.
    if args.iter().any(|arg| arg == "--shutdown") {
        app.exit(0);
        return;
    }
    if args.iter().any(|arg| arg == "--reset") {
        if let Some(engine) = app.try_state::<EngineState>() {
            engine.send(Command::EmergencyReset);
        }
        return;
    }
    app_windows::open_main_async(app);
}

/// Eventos del bucle principal.
pub fn on_run_event(app: &AppHandle, event: RunEvent) {
    match event {
        RunEvent::ExitRequested { code, api, .. } => {
            // Sin código: se cerró la última ventana. VRes sigue en la bandeja.
            // Con código: alguien pidió salir (bandeja, `--shutdown`, update).
            if code.is_none() {
                api.prevent_exit();
            }
        }
        RunEvent::Exit => shutdown(app),
        _ => {}
    }
}

/// Restaura todo lo que VRes tiene aplicado y apaga el motor. Es idempotente.
///
/// La llaman la salida normal (`RunEvent::Exit`) y el updater antes de
/// instalar (en Windows el updater termina el proceso sin pasar por `Exit`).
pub fn shutdown(app: &AppHandle) {
    static DONE: AtomicBool = AtomicBool::new(false);
    if DONE.swap(true, Ordering::SeqCst) {
        return;
    }
    tracing::info!("VRes cerrándose");
    if let Some(engine) = app.try_state::<EngineState>() {
        engine.shutdown();
    }
}
