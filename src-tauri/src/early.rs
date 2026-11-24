//! Flags que se atienden antes de arrancar Tauri.
//!
//! - `--version`: imprime la versión.
//! - `--shutdown`: cierra la instancia en ejecución (restaurando todo) y espera
//!   a que termine. Lo usa el instalador antes de reemplazar archivos.
//! - `--reset`: le pide a la instancia en ejecución que vuelva la pantalla a la
//!   normalidad. Lo usa el acceso directo "VRes — Restablecer pantalla".
//!
//! Ninguno de estos flags abre la interfaz.

#[cfg(windows)]
use std::time::Duration;

/// Si los argumentos piden una acción rápida, la ejecuta y devuelve el código
/// de salida. Si no, devuelve `None` y VRes arranca normalmente.
pub fn handle_early_flags() -> Option<i32> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let has = |flag: &str| args.iter().any(|arg| arg == flag);

    if has("--version") || has("-V") {
        #[cfg(windows)]
        vres_win::console::attach_parent_console();
        println!("{} {}", vres_core::APP_NAME, vres_core::APP_VERSION);
        return Some(0);
    }

    #[cfg(windows)]
    {
        use vres_win::control::{self, RemoteCommand};

        if has("--shutdown") {
            // Si no había instancia, o no terminó a tiempo, el instalador
            // igual verifica después si la app sigue abierta.
            let _ = control::shutdown_running(Duration::from_secs(3));
            return Some(0);
        }

        if has("--reset") {
            // Sin instancia en ejecución no hay cambios activos de esta sesión.
            // Desde F1, acá también se restaura lo que haya quedado en
            // `restore.json` tras un cierre inesperado.
            let _ = control::send(RemoteCommand::EmergencyReset, Duration::from_secs(2));
            return Some(0);
        }
    }

    None
}
