//! Guardián de rampas: toda prueba que toca una rampa de gamma la restaura al
//! terminar, aunque falle, haga `panic` o se interrumpa con `Ctrl + C`.

use std::sync::Mutex;

use vres_core::color::ramp::GammaRamp;
use vres_win::{WinResult, console, gamma};

/// Rampas originales pendientes de restaurar (para `Ctrl + C`).
static PENDING: Mutex<Vec<(String, GammaRamp)>> = Mutex::new(Vec::new());

/// Registra la restauración de emergencia ante `Ctrl + C`.
pub fn install_interrupt_handler() {
    console::on_interrupt(restore_pending);
}

fn restore_pending() {
    if let Ok(pending) = PENDING.lock() {
        for (gdi_name, ramp) in pending.iter() {
            let _ = gamma::write_ramp(gdi_name, ramp);
        }
    }
}

/// Captura la rampa original de una pantalla y la restaura al destruirse.
#[derive(Debug)]
pub struct RampGuard {
    gdi_name: String,
    original: GammaRamp,
}

impl RampGuard {
    /// Lee y guarda la rampa vigente de `gdi_name`.
    pub fn capture(gdi_name: &str) -> WinResult<Self> {
        let original = gamma::read_ramp(gdi_name)?;
        if let Ok(mut pending) = PENDING.lock() {
            pending.push((gdi_name.to_owned(), original.clone()));
        }
        Ok(Self {
            gdi_name: gdi_name.to_owned(),
            original,
        })
    }

    /// La rampa que había antes de la prueba.
    pub fn original(&self) -> &GammaRamp {
        &self.original
    }
}

impl Drop for RampGuard {
    fn drop(&mut self) {
        let _ = gamma::write_ramp(&self.gdi_name, &self.original);
        if let Ok(mut pending) = PENDING.lock()
            && let Some(position) = pending.iter().position(|(name, _)| *name == self.gdi_name)
        {
            pending.remove(position);
        }
    }
}
