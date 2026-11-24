//! Looks incluidos en VRes: puntos de partida propios, generales y por juego.
//!
//! Son valores de VRes (no copiados de ningún otro producto) y se pensaron
//! para ajustar a ojo: cada monitor y cada vista es distinta. La interfaz
//! traduce el nombre por `id` (`looks.presets.<id>`).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::model::look::{EyeCare, Look, Tone};

/// Juegos con looks y, más adelante, perfiles propios.
pub const GAMES: &[(&str, &str)] = &[
    ("fortnite", "Fortnite"),
    ("valorant", "VALORANT"),
    ("cs2", "Counter-Strike 2"),
    ("apex", "Apex Legends"),
    ("rust", "Rust"),
    ("r6", "Rainbow Six Siege"),
];

/// Un look incluido.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LookPreset {
    /// Identificador estable (la interfaz traduce el nombre).
    pub id: String,
    /// Juego para el que se pensó (`None`: sirve para cualquiera).
    pub game: Option<String>,
    pub look: Look,
}

struct Spec {
    id: &'static str,
    game: Option<&'static str>,
    saturation: f32,
    vibrance: f32,
    contrast: f32,
    gamma: f32,
    shadows: f32,
    highlights: f32,
    blacks: f32,
    warmth: f32,
}

const NEUTRAL: Spec = Spec {
    id: "",
    game: None,
    saturation: 100.0,
    vibrance: 50.0,
    contrast: 100.0,
    gamma: 1.0,
    shadows: 0.0,
    highlights: 0.0,
    blacks: 0.0,
    warmth: 0.0,
};

const SPECS: &[Spec] = &[
    Spec {
        id: "vivid",
        saturation: 140.0,
        vibrance: 60.0,
        ..NEUTRAL
    },
    Spec {
        id: "clarity",
        saturation: 115.0,
        contrast: 112.0,
        shadows: 15.0,
        highlights: -10.0,
        ..NEUTRAL
    },
    Spec {
        id: "night",
        saturation: 110.0,
        gamma: 1.12,
        shadows: 35.0,
        blacks: 20.0,
        ..NEUTRAL
    },
    Spec {
        id: "soft",
        saturation: 92.0,
        contrast: 96.0,
        warmth: 30.0,
        ..NEUTRAL
    },
    Spec {
        id: "fortnite-vivid",
        game: Some("fortnite"),
        saturation: 150.0,
        vibrance: 65.0,
        contrast: 105.0,
        ..NEUTRAL
    },
    Spec {
        id: "fortnite-clean",
        game: Some("fortnite"),
        saturation: 120.0,
        shadows: 10.0,
        highlights: -5.0,
        ..NEUTRAL
    },
    Spec {
        id: "valorant-pop",
        game: Some("valorant"),
        saturation: 135.0,
        contrast: 110.0,
        shadows: 12.0,
        ..NEUTRAL
    },
    Spec {
        id: "valorant-clean",
        game: Some("valorant"),
        saturation: 115.0,
        gamma: 1.05,
        ..NEUTRAL
    },
    Spec {
        id: "cs2-visibility",
        game: Some("cs2"),
        saturation: 125.0,
        gamma: 1.08,
        shadows: 25.0,
        blacks: 15.0,
        ..NEUTRAL
    },
    Spec {
        id: "cs2-sharp",
        game: Some("cs2"),
        saturation: 130.0,
        contrast: 115.0,
        ..NEUTRAL
    },
    Spec {
        id: "apex-vivid",
        game: Some("apex"),
        saturation: 140.0,
        vibrance: 60.0,
        shadows: 15.0,
        ..NEUTRAL
    },
    Spec {
        id: "apex-clean",
        game: Some("apex"),
        saturation: 115.0,
        contrast: 105.0,
        ..NEUTRAL
    },
    Spec {
        id: "rust-night",
        game: Some("rust"),
        saturation: 115.0,
        gamma: 1.15,
        shadows: 40.0,
        blacks: 25.0,
        ..NEUTRAL
    },
    Spec {
        id: "rust-day",
        game: Some("rust"),
        saturation: 125.0,
        highlights: -10.0,
        ..NEUTRAL
    },
    Spec {
        id: "r6-contrast",
        game: Some("r6"),
        saturation: 120.0,
        contrast: 112.0,
        shadows: 20.0,
        ..NEUTRAL
    },
    Spec {
        id: "r6-vivid",
        game: Some("r6"),
        saturation: 140.0,
        vibrance: 60.0,
        ..NEUTRAL
    },
];

/// Los looks incluidos en VRes.
pub fn presets() -> Vec<LookPreset> {
    SPECS
        .iter()
        .map(|spec| LookPreset {
            id: spec.id.to_owned(),
            game: spec.game.map(str::to_owned),
            look: Look {
                saturation: spec.saturation,
                vibrance: spec.vibrance,
                tone: Tone {
                    contrast: spec.contrast,
                    gamma: spec.gamma,
                    shadows: spec.shadows,
                    highlights: spec.highlights,
                    blacks: spec.blacks,
                    ..Tone::default()
                },
                eye_care: EyeCare {
                    warmth: spec.warmth,
                    ..EyeCare::default()
                },
                ..Look::default()
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_valid_and_unique() {
        let list = presets();
        let mut ids: Vec<&str> = list.iter().map(|preset| preset.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), list.len(), "sin ids repetidos");
        for preset in &list {
            assert_eq!(
                preset.look.clamped(),
                preset.look,
                "{} está dentro de los rangos",
                preset.id
            );
            assert_ne!(preset.look, Look::default(), "{} cambia algo", preset.id);
            if let Some(game) = &preset.game {
                assert!(
                    GAMES.iter().any(|(id, _)| id == game),
                    "{}: juego desconocido",
                    preset.id
                );
            }
        }
        for (game, _) in GAMES {
            assert!(
                list.iter()
                    .filter(|preset| preset.game.as_deref() == Some(*game))
                    .count()
                    >= 2,
                "{game} tiene al menos dos looks"
            );
        }
    }

    #[test]
    fn presets_do_not_mention_other_products() {
        let text = serde_json::to_string(&presets())
            .expect("serializable")
            .to_lowercase();
        for name in ["plexus", "spectra", "vibrancegui"] {
            assert!(
                !text.contains(name),
                "los presets son propios: aparece {name}"
            );
        }
    }
}
