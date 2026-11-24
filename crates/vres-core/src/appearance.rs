//! Aspecto y comportamiento de la app: tema, fondo propio y qué pasa al
//! cerrar la ventana.

use serde::{Deserialize, Deserializer, Serialize};
use ts_rs::TS;

/// Tamaño máximo de la imagen de fondo.
pub const MAX_BACKGROUND_BYTES: usize = 25 * 1024 * 1024;

/// Tema de la interfaz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Theme {
    /// Negro total con el verde lima de VRes.
    #[default]
    Black,
    /// Superficies grafito, mismo acento.
    Graphite,
}

impl<'de> Deserialize<'de> for Theme {
    /// Un tema desconocido (de otra versión) vuelve al de siempre en vez de
    /// invalidar todos los ajustes.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        Ok(match name.as_str() {
            "graphite" => Self::Graphite,
            _ => Self::Black,
        })
    }
}

/// Color de acento de la interfaz (el verde lima es el de VRes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Accent {
    #[default]
    Lime,
    Cyan,
    Blue,
    Violet,
    Pink,
    Red,
    Orange,
    Yellow,
}

impl Accent {
    /// Todos, en el orden en que se muestran.
    pub const ALL: [Self; 8] = [
        Self::Lime,
        Self::Cyan,
        Self::Blue,
        Self::Violet,
        Self::Pink,
        Self::Red,
        Self::Orange,
        Self::Yellow,
    ];
}

impl<'de> Deserialize<'de> for Accent {
    /// Un acento desconocido vuelve al verde lima.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        Ok(match name.as_str() {
            "cyan" => Self::Cyan,
            "blue" => Self::Blue,
            "violet" => Self::Violet,
            "pink" => Self::Pink,
            "red" => Self::Red,
            "orange" => Self::Orange,
            "yellow" => Self::Yellow,
            _ => Self::Lime,
        })
    }
}

/// Tamaños de la interfaz permitidos (%).
pub const UI_SCALES: [u32; 4] = [90, 100, 110, 125];
/// Desenfoque máximo del fondo (px).
pub const MAX_BACKGROUND_BLUR: f32 = 24.0;

/// La imagen de fondo guardada (`media/background.bin`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BackgroundImage {
    /// `image/png`, `image/jpeg`, `image/gif` o `image/webp`.
    pub mime: String,
    /// Cambia con cada imagen nueva (para que la interfaz la vuelva a leer).
    pub revision: u32,
}

/// Aspecto de VRes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct AppearanceSettings {
    pub theme: Theme,
    pub accent: Accent,
    /// Tamaño de la interfaz (%): 90, 100, 110 o 125.
    pub ui_scale: u32,
    /// Imagen o GIF de fondo (`None`: sin fondo). Solo la cambia
    /// `customize_background_set`.
    pub background: Option<BackgroundImage>,
    /// Cuánto se ve el fondo (0–100).
    pub background_opacity: f32,
    /// Desenfoque del fondo (0–24 px).
    pub background_blur: f32,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: Theme::Black,
            accent: Accent::Lime,
            ui_scale: 100,
            background: None,
            background_opacity: 25.0,
            background_blur: 0.0,
        }
    }
}

impl AppearanceSettings {
    /// El mismo con valores válidos.
    pub fn sanitized(mut self) -> Self {
        self.background_opacity = if self.background_opacity.is_finite() {
            self.background_opacity.clamp(0.0, 100.0)
        } else {
            25.0
        };
        self.background_blur = if self.background_blur.is_finite() {
            self.background_blur.clamp(0.0, MAX_BACKGROUND_BLUR)
        } else {
            0.0
        };
        // El tamaño permitido más cercano.
        self.ui_scale = UI_SCALES
            .into_iter()
            .min_by_key(|scale| scale.abs_diff(self.ui_scale))
            .unwrap_or(100);
        if self
            .background
            .as_ref()
            .is_some_and(|image| !ALLOWED_MIMES.contains(&image.mime.as_str()))
        {
            self.background = None;
        }
        self
    }
}

/// Comportamiento de la app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct BehaviorSettings {
    /// Al cerrar la ventana, VRes sigue en la bandeja; si no, se cierra (y
    /// la pantalla vuelve a la normalidad).
    pub close_to_tray: bool,
}

impl Default for BehaviorSettings {
    fn default() -> Self {
        Self {
            close_to_tray: true,
        }
    }
}

const ALLOWED_MIMES: [&str; 4] = ["image/png", "image/jpeg", "image/gif", "image/webp"];

/// Tipo de una imagen por sus primeros bytes (no por la extensión).
pub fn image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn images_are_recognized_by_their_bytes() {
        assert_eq!(image_mime(b"\x89PNG\r\n\x1a\n...."), Some("image/png"));
        assert_eq!(image_mime(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
        assert_eq!(image_mime(b"GIF89a...."), Some("image/gif"));
        assert_eq!(image_mime(b"RIFF\0\0\0\0WEBPVP8 "), Some("image/webp"));
        assert_eq!(
            image_mime(b"<svg xmlns="),
            None,
            "SVG no: puede tener scripts"
        );
        assert_eq!(image_mime(b"MZ\x90\0"), None);
        assert_eq!(image_mime(b""), None);
    }

    #[test]
    fn an_unknown_theme_falls_back_instead_of_failing() {
        let parsed: AppearanceSettings =
            serde_json::from_str(r#"{"theme":"neon","backgroundOpacity":250}"#).expect("válido");
        assert_eq!(parsed.theme, Theme::Black);
        assert_eq!(parsed.sanitized().background_opacity, 100.0);
        let graphite: Theme = serde_json::from_str(r#""graphite""#).expect("válido");
        assert_eq!(graphite, Theme::Graphite);
    }

    #[test]
    fn accent_scale_and_blur_stay_valid() {
        let parsed: AppearanceSettings =
            serde_json::from_str(r#"{"accent":"magenta","uiScale":300,"backgroundBlur":-4}"#)
                .expect("válido");
        let clean = parsed.sanitized();
        assert_eq!(
            clean.accent,
            Accent::Lime,
            "un acento desconocido vuelve al lima"
        );
        assert_eq!(clean.ui_scale, 125, "el tamaño más cercano");
        assert_eq!(clean.background_blur, 0.0);
        let violet: Accent = serde_json::from_str(r#""violet""#).expect("válido");
        assert_eq!(violet, Accent::Violet);
        assert_eq!(
            serde_json::to_string(&Accent::Pink).expect("json"),
            r#""pink""#
        );
        let old: AppearanceSettings =
            serde_json::from_str(r#"{"theme":"graphite"}"#).expect("válido");
        assert_eq!(
            old.ui_scale, 100,
            "los ajustes viejos toman los valores nuevos por defecto"
        );
    }

    #[test]
    fn only_known_image_types_are_kept() {
        let settings = AppearanceSettings {
            background: Some(BackgroundImage {
                mime: "text/html".into(),
                revision: 1,
            }),
            ..AppearanceSettings::default()
        };
        assert_eq!(settings.sanitized().background, None);
        assert!(BehaviorSettings::default().close_to_tray);
    }
}
