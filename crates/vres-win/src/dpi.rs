//! Conciencia de DPI.
//!
//! La app de Tauri ya la configura (tao usa "por monitor v2"). Las
//! herramientas de consola tienen que hacerlo ellas mismas para trabajar en
//! píxeles físicos.

use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};

/// Configura el proceso como "consciente del DPI por monitor (v2)".
///
/// Devuelve `false` si ya estaba configurado de otra forma.
pub fn make_process_per_monitor_aware() -> bool {
    // SAFETY: la llamada solo cambia un ajuste del proceso; si ya estaba
    // configurado, devuelve error sin efectos.
    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }.is_ok()
}
