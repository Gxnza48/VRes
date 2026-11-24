//! Consola: flags de la app gráfica y `Ctrl + C` en las herramientas.

use std::sync::OnceLock;

use windows::Win32::System::Console::{
    ATTACH_PARENT_PROCESS, AttachConsole, SetConsoleCtrlHandler,
};
use windows::core::BOOL;

/// Engancha el proceso a la consola de su padre, si tiene una.
///
/// `vres.exe` es una app de ventana (subsistema `windows`): sin esto,
/// `vres.exe --version` no imprimiría nada en la terminal que lo llamó.
pub fn attach_parent_console() -> bool {
    // SAFETY: AttachConsole no recibe punteros; si el padre no tiene consola,
    // simplemente devuelve error y no cambia nada.
    unsafe { AttachConsole(ATTACH_PARENT_PROCESS) }.is_ok()
}

static INTERRUPT_HANDLER: OnceLock<fn()> = OnceLock::new();

extern "system" fn on_console_event(_kind: u32) -> BOOL {
    if let Some(handler) = INTERRUPT_HANDLER.get() {
        handler();
    }
    // FALSE: después de restaurar, sigue el comportamiento normal (terminar).
    BOOL(0)
}

/// Registra una función que se llama ante `Ctrl + C`, `Ctrl + Break` o el
/// cierre de la consola, antes de que el proceso termine. Solo se puede
/// registrar una vez.
pub fn on_interrupt(handler: fn()) -> bool {
    if INTERRUPT_HANDLER.set(handler).is_err() {
        return false;
    }
    // SAFETY: `on_console_event` es una función estática válida durante toda
    // la vida del proceso.
    unsafe { SetConsoleCtrlHandler(Some(on_console_event), true) }.is_ok()
}
