//! Consola para los flags de línea de comandos de la app gráfica.
//!
//! `vres.exe` es una app de ventana (subsistema `windows`): no tiene consola.
//! Para que `vres.exe --version` imprima algo en la terminal desde la que se
//! lo llamó, se engancha a la consola del proceso padre.

use windows::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};

/// Engancha el proceso a la consola de su padre, si tiene una.
///
/// Devuelve `true` si a partir de ahora `println!` se ve en esa consola.
pub fn attach_parent_console() -> bool {
    // SAFETY: AttachConsole no recibe punteros; si el padre no tiene consola,
    // simplemente devuelve error y no cambia nada.
    unsafe { AttachConsole(ATTACH_PARENT_PROCESS) }.is_ok()
}
