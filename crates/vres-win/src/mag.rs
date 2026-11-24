//! Matriz de color y zoom de pantalla completa (Magnification API).
//!
//! El efecto aplica a **todo el escritorio** y Windows lo compone con DWM: no
//! se ve en pantalla completa exclusiva. No funciona bajo WOW64 (VRes es solo
//! de 64 bits).

use std::marker::PhantomData;

use vres_core::color::matrix::ColorMatrix;
use windows::Win32::UI::Magnification::{
    MAGCOLOREFFECT, MagGetFullscreenColorEffect, MagGetFullscreenTransform, MagInitialize,
    MagSetFullscreenColorEffect, MagSetFullscreenTransform, MagUninitialize,
};

use crate::{WinError, WinResult};

// La estructura que espera Windows: 25 floats, exactamente 100 bytes.
const _: () = assert!(size_of::<MAGCOLOREFFECT>() == 100);

/// Sesión de la Magnification API.
///
/// Debe usarse siempre desde el hilo que la creó (por eso no es `Send`). Al
/// destruirse llama a `MagUninitialize`, que quita el efecto.
#[derive(Debug)]
pub struct Magnifier {
    _thread_bound: PhantomData<*const ()>,
}

impl Magnifier {
    /// Inicia la sesión.
    pub fn init() -> WinResult<Self> {
        // SAFETY: MagInitialize no recibe parámetros.
        unsafe { MagInitialize() }
            .ok()
            .map_err(|e| WinError::api("MagInitialize", e))?;
        Ok(Self {
            _thread_bound: PhantomData,
        })
    }

    /// Aplica una matriz de color a todo el escritorio.
    pub fn set_color_effect(&self, matrix: &ColorMatrix) -> WinResult<()> {
        let effect = MAGCOLOREFFECT {
            transform: matrix.to_flat(),
        };
        // SAFETY: `effect` es una estructura válida que vive durante la llamada.
        unsafe { MagSetFullscreenColorEffect(&effect) }
            .ok()
            .map_err(|e| WinError::api("MagSetFullscreenColorEffect", e))
    }

    /// Lee la matriz de color vigente.
    pub fn color_effect(&self) -> WinResult<ColorMatrix> {
        let mut effect = MAGCOLOREFFECT::default();
        // SAFETY: `effect` es memoria válida y escribible durante la llamada.
        unsafe { MagGetFullscreenColorEffect(&mut effect) }
            .ok()
            .map_err(|e| WinError::api("MagGetFullscreenColorEffect", e))?;
        Ok(ColorMatrix::from_flat(effect.transform))
    }

    /// Amplía la pantalla `level` veces. Los desplazamientos son relativos a la
    /// esquina del monitor principal, en coordenadas sin ampliar.
    pub fn set_transform(&self, level: f32, x_offset: i32, y_offset: i32) -> WinResult<()> {
        // SAFETY: solo recibe valores.
        unsafe { MagSetFullscreenTransform(level, x_offset, y_offset) }
            .ok()
            .map_err(|e| WinError::api("MagSetFullscreenTransform", e))
    }

    /// Lee la ampliación vigente: nivel y desplazamientos.
    pub fn transform(&self) -> WinResult<(f32, i32, i32)> {
        let (mut level, mut x, mut y) = (0.0f32, 0i32, 0i32);
        // SAFETY: las tres variables viven durante la llamada.
        unsafe { MagGetFullscreenTransform(&mut level, &mut x, &mut y) }
            .ok()
            .map_err(|e| WinError::api("MagGetFullscreenTransform", e))?;
        Ok((level, x, y))
    }
}

impl Drop for Magnifier {
    fn drop(&mut self) {
        // SAFETY: cierra la sesión que abrimos en `init`, en el mismo hilo.
        let _ = unsafe { MagUninitialize() };
    }
}
