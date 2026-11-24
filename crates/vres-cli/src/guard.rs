//! Guardianes: toda prueba que toca una rampa de gamma, la resolución o el HDR
//! la restaura al terminar, aunque falle, haga `panic` o se interrumpa con
//! `Ctrl + C`.

use std::sync::Mutex;

use vres_core::color::ramp::GammaRamp;
use vres_win::{WinResult, console, gamma, modes};

/// Rampas originales pendientes de restaurar (para `Ctrl + C`).
static PENDING: Mutex<Vec<(String, GammaRamp)>> = Mutex::new(Vec::new());
/// Pantallas con un modo de prueba aplicado.
static PENDING_MODES: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// Pantallas con el HDR cambiado, y su estado original.
static PENDING_HDR: Mutex<Vec<(String, bool)>> = Mutex::new(Vec::new());

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
    if let Ok(pending) = PENDING_MODES.lock() {
        for gdi_name in pending.iter() {
            let _ = modes::revert_to_saved(gdi_name);
        }
    }
    if let Ok(pending) = PENDING_HDR.lock() {
        for (gdi_name, enabled) in pending.iter() {
            let _ = modes::set_hdr(gdi_name, *enabled);
        }
    }
}

fn forget<T>(list: &Mutex<Vec<T>>, matches: impl Fn(&T) -> bool) {
    if let Ok(mut pending) = list.lock()
        && let Some(position) = pending.iter().position(matches)
    {
        pending.remove(position);
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
        forget(&PENDING, |(name, _)| *name == self.gdi_name);
    }
}

/// Vuelve al modo guardado en Windows al destruirse.
#[derive(Debug)]
pub struct ModeGuard {
    gdi_name: String,
    armed: bool,
}

impl ModeGuard {
    /// Se arma antes de aplicar un modo de prueba.
    pub fn arm(gdi_name: &str) -> Self {
        if let Ok(mut pending) = PENDING_MODES.lock() {
            pending.push(gdi_name.to_owned());
        }
        Self {
            gdi_name: gdi_name.to_owned(),
            armed: true,
        }
    }

    /// Vuelve ahora y devuelve el resultado.
    pub fn revert(mut self) -> WinResult<()> {
        self.armed = false;
        forget(&PENDING_MODES, |name| *name == self.gdi_name);
        modes::revert_to_saved(&self.gdi_name)
    }
}

impl Drop for ModeGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = modes::revert_to_saved(&self.gdi_name);
            forget(&PENDING_MODES, |name| *name == self.gdi_name);
        }
    }
}

/// Deja el HDR como estaba al destruirse.
#[derive(Debug)]
pub struct HdrGuard {
    gdi_name: String,
    original: bool,
    armed: bool,
}

impl HdrGuard {
    /// Se arma antes de cambiar el HDR, con su estado original.
    pub fn arm(gdi_name: &str, original: bool) -> Self {
        if let Ok(mut pending) = PENDING_HDR.lock() {
            pending.push((gdi_name.to_owned(), original));
        }
        Self {
            gdi_name: gdi_name.to_owned(),
            original,
            armed: true,
        }
    }

    /// Restaura ahora y devuelve el resultado.
    pub fn restore(mut self) -> WinResult<()> {
        self.armed = false;
        forget(&PENDING_HDR, |(name, _)| *name == self.gdi_name);
        modes::set_hdr(&self.gdi_name, self.original)
    }
}

impl Drop for HdrGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = modes::set_hdr(&self.gdi_name, self.original);
            forget(&PENDING_HDR, |(name, _)| *name == self.gdi_name);
        }
    }
}
