//! # vres-core
//!
//! Lógica pura de VRes. Este crate no sabe que existe Windows ni Tauri: por eso
//! se puede testear en cualquier sistema operativo y en CI sin pantalla.
//!
//! Contiene los modelos de datos, la matemática de color, el planificador y los
//! tipos que cruzan la frontera con la interfaz (IPC). Los tipos marcados con
//! `#[ts(export)]` se exportan a TypeScript en `src/lib/bindings` al correr
//! `cargo test`.
#![forbid(unsafe_code)]

pub mod appearance;
pub mod color;
pub mod crosshair;
pub mod enhancers;
pub mod game_codes;
pub mod game_tune;
pub mod ipc;
pub mod keys;
pub mod looks;
pub mod model;
pub mod monitor;
pub mod planner;
pub mod potato;
pub mod profiles;
pub mod settings;
pub mod share;
pub mod status;
pub mod store;

/// Identificador de la aplicación: carpetas de datos, instalador y WebView2.
pub const APP_IDENTIFIER: &str = "io.github.gxnza48.vres";

/// Nombre visible de la aplicación.
pub const APP_NAME: &str = "VRes";

/// Versión de VRes (la del workspace de Cargo).
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Repositorio público.
pub const REPOSITORY_URL: &str = "https://github.com/Gxnza48/VRes";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifier_is_reverse_domain() {
        assert!(APP_IDENTIFIER.starts_with("io.github."));
        assert!(!APP_IDENTIFIER.contains(char::is_whitespace));
    }

    #[test]
    fn version_is_semver_like() {
        let parts: Vec<_> = APP_VERSION.split('.').collect();
        assert_eq!(parts.len(), 3, "versión inesperada: {APP_VERSION}");
        assert!(parts.iter().all(|p| p.parse::<u32>().is_ok()));
    }
}
