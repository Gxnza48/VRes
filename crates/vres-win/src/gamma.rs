//! Rampas de gamma por monitor (`GetDeviceGammaRamp` / `SetDeviceGammaRamp`).
//!
//! Microsoft documenta que `SetDeviceGammaRamp` puede devolver éxito **sin**
//! aplicar la rampa: por eso toda escritura se verifica releyendo.

use std::ffi::c_void;
use std::time::{Duration, Instant};

use vres_core::color::ramp::GammaRamp;
use windows::Win32::Graphics::Gdi::{CreateDCW, DeleteDC, HDC};
use windows::Win32::UI::ColorSystem::{GetDeviceGammaRamp, SetDeviceGammaRamp};
use windows::core::PCWSTR;

use crate::{WinError, WinResult};

/// Contexto de dispositivo de una pantalla, liberado al salir de alcance.
struct DisplayDc(HDC);

impl DisplayDc {
    fn open(gdi_name: &str) -> WinResult<Self> {
        let wide: Vec<u16> = gdi_name.encode_utf16().chain(Some(0)).collect();
        // SAFETY: `wide` es una cadena UTF-16 terminada en NUL que vive durante
        // la llamada; para una pantalla, driver y dispositivo son el mismo nombre.
        let hdc = unsafe {
            CreateDCW(
                PCWSTR(wide.as_ptr()),
                PCWSTR(wide.as_ptr()),
                PCWSTR::null(),
                None,
            )
        };
        if hdc.is_invalid() {
            Err(WinError::last("CreateDCW"))
        } else {
            Ok(Self(hdc))
        }
    }
}

impl Drop for DisplayDc {
    fn drop(&mut self) {
        // SAFETY: el DC lo creamos nosotros y se libera una sola vez.
        let _ = unsafe { DeleteDC(self.0) };
    }
}

/// Lee la rampa vigente de una pantalla (`\\.\DISPLAYn`).
pub fn read_ramp(gdi_name: &str) -> WinResult<GammaRamp> {
    let dc = DisplayDc::open(gdi_name)?;
    let mut ramp = GammaRamp::identity();
    // SAFETY: `ramp.channels` tiene exactamente el tamaño de WORD[3][256].
    let ok = unsafe { GetDeviceGammaRamp(dc.0, ramp.channels.as_mut_ptr().cast::<c_void>()) };
    if ok.as_bool() {
        Ok(ramp)
    } else {
        Err(WinError::last("GetDeviceGammaRamp"))
    }
}

/// Escribe una rampa. Devuelve `true` si Windows dijo que la aceptó (lo que
/// no garantiza que la haya aplicado: usar [`write_ramp_verified`]).
pub fn write_ramp(gdi_name: &str, ramp: &GammaRamp) -> WinResult<bool> {
    let dc = DisplayDc::open(gdi_name)?;
    // SAFETY: `ramp.channels` tiene exactamente el tamaño de WORD[3][256] y
    // vive durante la llamada.
    let ok = unsafe { SetDeviceGammaRamp(dc.0, ramp.channels.as_ptr().cast::<c_void>()) };
    Ok(ok.as_bool())
}

/// Resultado de una escritura verificada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RampWrite {
    /// Windows dijo que la aceptó.
    pub accepted: bool,
    /// La rampa releída coincide con la pedida (dentro de la tolerancia).
    pub verified: bool,
    /// Mayor diferencia entre lo pedido y lo releído.
    pub max_diff: u16,
    /// Cuánto tardó la escritura.
    pub write_time: Duration,
}

/// Escribe una rampa y la relee para confirmar que se aplicó.
pub fn write_ramp_verified(
    gdi_name: &str,
    ramp: &GammaRamp,
    tolerance: u16,
) -> WinResult<RampWrite> {
    let started = Instant::now();
    let accepted = write_ramp(gdi_name, ramp)?;
    let write_time = started.elapsed();
    let current = read_ramp(gdi_name)?;
    let max_diff = current.max_abs_diff(ramp);
    Ok(RampWrite {
        accepted,
        verified: max_diff <= tolerance,
        max_diff,
        write_time,
    })
}
