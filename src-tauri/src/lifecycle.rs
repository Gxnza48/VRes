//! Ciclo de vida: arranque, segunda instancia, cierre de ventanas y salida.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{App, AppHandle, Manager, RunEvent};

use crate::{AUTOSTART_FLAG, LaunchInfo, app_windows, logging, tray};

#[cfg(windows)]
use std::sync::Mutex;
#[cfg(windows)]
use vres_win::control::{ACK, ControlWindow, RemoteCommand};

/// Ventana de control: recibe `--shutdown` y `--reset` desde otros procesos.
#[cfg(windows)]
struct ControlState(Mutex<Option<ControlWindow>>);

/// Inicialización, antes de que corra el bucle de eventos.
pub fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();
    logging::init(&handle);
    tracing::info!(version = vres_core::APP_VERSION, "VRes iniciando");

    let autostarted = std::env::args().any(|arg| arg == AUTOSTART_FLAG);
    app.manage(LaunchInfo { autostarted });

    #[cfg(windows)]
    {
        let control = start_control_window(handle.clone())?;
        app.manage(ControlState(Mutex::new(Some(control))));
    }

    tray::create(&handle)?;

    if !autostarted {
        // En `setup` todavía no corre el bucle de eventos: crear la ventana
        // acá es seguro (no es un handler síncrono).
        app_windows::open_main(&handle);
    }
    Ok(())
}

#[cfg(windows)]
fn start_control_window(app: AppHandle) -> Result<ControlWindow, vres_win::WinError> {
    ControlWindow::spawn(Box::new(move |command| {
        match command {
            RemoteCommand::Shutdown => {
                tracing::info!("cierre pedido desde otro proceso");
                // No bloquea: le pide al bucle de eventos que salga.
                app.exit(0);
            }
            RemoteCommand::EmergencyReset => {
                // F1: el motor vuelve todas las capas a su estado original.
                tracing::info!("restablecimiento pedido desde otro proceso");
            }
            RemoteCommand::Ping => {}
        }
        ACK
    }))
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
        tracing::info!("restablecimiento pedido por una segunda instancia");
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

/// Apaga todo lo que VRes tiene activo. Es idempotente.
///
/// La llaman la salida normal (`RunEvent::Exit`) y el updater antes de
/// instalar (en Windows el updater termina el proceso sin pasar por `Exit`).
pub fn shutdown(app: &AppHandle) {
    static DONE: AtomicBool = AtomicBool::new(false);
    if DONE.swap(true, Ordering::SeqCst) {
        return;
    }
    tracing::info!("VRes cerrándose");

    // F1: acá se apaga el motor, que restaura rampas, vibrance y resolución.

    #[cfg(windows)]
    if let Some(state) = app.try_state::<ControlState>() {
        let control = state.0.lock().ok().and_then(|mut guard| guard.take());
        if let Some(control) = control {
            control.close();
        }
    }
    #[cfg(not(windows))]
    let _ = app;
}
