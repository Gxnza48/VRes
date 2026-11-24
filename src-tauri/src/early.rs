//! Flags que se atienden antes de arrancar Tauri.
//!
//! - `--version`: imprime la versión.
//! - `--shutdown`: cierra la instancia en ejecución (restaurando todo) y espera
//!   a que termine. Lo usa el instalador antes de reemplazar archivos.
//! - `--reset`: vuelve la pantalla a la normalidad. Si VRes está corriendo, se
//!   lo pide; si no, deshace lo que haya quedado de un cierre inesperado. Lo
//!   usa el acceso directo "VRes — Restablecer pantalla".
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
            match control::send(RemoteCommand::EmergencyReset, Duration::from_secs(2)) {
                Ok(Some(_)) => {}
                _ => {
                    vres_engine::recover_from_disk();
                }
            }
            return Some(0);
        }
    }

    None
}
