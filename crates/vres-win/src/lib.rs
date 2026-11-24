//! # vres-win
//!
//! Acceso a Windows y a los drivers de video, detrás de APIs seguras.
//!
//! Es el **único** crate del workspace con `unsafe`. Reglas:
//! - cada bloque `unsafe` lleva un comentario `// SAFETY:` que explica por qué
//!   es correcto (clippy lo exige con `undocumented_unsafe_blocks`);
//! - ninguna función pública es `unsafe`: quien usa este crate no necesita
//!   saber nada de punteros ni de handles;
//! - nunca se abre un handle al proceso de un juego, nunca se instala un hook
//!   de teclado o mouse de bajo nivel y nunca se inyecta nada.
#![cfg(windows)]
#![deny(clippy::unwrap_used)]

pub mod console;
pub mod control;
pub mod displays;
pub mod dpi;
pub mod error;
pub mod gamma;
pub mod identify;
pub mod keyboard;
pub mod mag;
pub mod modes;
pub mod overlay;
pub mod probe;
pub mod process;
pub mod sound;
pub mod vibrance;
pub mod window;

pub use error::{WinError, WinResult};
