//! Perfiles por juego: qué juego está al frente y qué aplicar.
//!
//! Reglas de la sesión (función pura [`resolve`]):
//! - el **look** y el **crosshair** del perfil valen solo mientras el juego
//!   está en primer plano: con Alt+Tab vuelve lo del escritorio;
//! - la **resolución** se aplica al entrar al juego y se revierte recién al
//!   cerrarlo (no en cada Alt+Tab);
//! - con el cambio automático apagado, no se aplica nada.
//!
//! Los juegos se reconocen por el nombre de su ejecutable, sin abrir nunca su
//! proceso.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::model::look::Look;
use crate::settings::SavedMode;

/// Un juego conocido.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownGame {
    pub id: &'static str,
    pub name: &'static str,
    /// Ejecutables del juego (no del launcher).
    pub executables: &'static [&'static str],
    /// Anti-cheat, para los avisos.
    pub anti_cheat: &'static str,
}

/// Los seis juegos con soporte desde el inicio. Los nombres de ejecutable se
/// confirman en una PC real antes de marcarlos como verificados.
pub const KNOWN_GAMES: &[KnownGame] = &[
    KnownGame {
        id: "fortnite",
        name: "Fortnite",
        executables: &["FortniteClient-Win64-Shipping.exe"],
        anti_cheat: "Easy Anti-Cheat",
    },
    KnownGame {
        id: "valorant",
        name: "VALORANT",
        executables: &["VALORANT-Win64-Shipping.exe"],
        anti_cheat: "Vanguard",
    },
    KnownGame {
        id: "cs2",
        name: "Counter-Strike 2",
        executables: &["cs2.exe"],
        anti_cheat: "VAC",
    },
    KnownGame {
        id: "apex",
        name: "Apex Legends",
        executables: &["r5apex.exe", "r5apex_dx12.exe"],
        anti_cheat: "EA Javelin",
    },
    KnownGame {
        id: "rust",
        name: "Rust",
        executables: &["RustClient.exe"],
        anti_cheat: "Easy Anti-Cheat",
    },
    KnownGame {
        id: "r6",
        name: "Rainbow Six Siege",
        executables: &[
            "RainbowSix.exe",
            "RainbowSix_Vulkan.exe",
            "RainbowSix_BE.exe",
        ],
        anti_cheat: "BattlEye",
    },
];

/// Un juego que agregó el usuario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CustomGame {
    /// Identificador estable (`custom:` + ejecutable en minúsculas).
    pub id: String,
    pub name: String,
    /// Nombre del ejecutable, por ejemplo `MiJuego.exe`.
    pub executable: String,
}

/// Qué hacer con el crosshair mientras el juego está al frente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CrosshairChoice {
    /// Como esté en la pestaña Crosshair.
    #[default]
    Keep,
    /// Prendido.
    On,
    /// Apagado.
    Off,
}

/// El perfil de un juego.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameProfile {
    pub game_id: String,
    pub enabled: bool,
    /// Look mientras el juego está al frente (`None`: el del escritorio).
    pub look: Option<Look>,
    pub crosshair: CrosshairChoice,
    /// Resolución mientras el juego está abierto (tiene que estar conservada).
    pub resolution: Option<SavedMode>,
}

/// Ajustes de perfiles.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ProfileSettings {
    /// Cambiar solo al detectar el juego.
    pub auto_switch: bool,
    pub profiles: Vec<GameProfile>,
    pub custom_games: Vec<CustomGame>,
}

/// Solo el nombre del archivo, sin la carpeta.
fn basename(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

/// El juego de un ejecutable (sin distinguir mayúsculas), o `None`.
pub fn match_game(executable: &str, custom: &[CustomGame]) -> Option<String> {
    let exe = basename(executable);
    if let Some(game) = KNOWN_GAMES.iter().find(|game| {
        game.executables
            .iter()
            .any(|known| known.eq_ignore_ascii_case(exe))
    }) {
        return Some(game.id.to_owned());
    }
    custom
        .iter()
        .find(|game| basename(&game.executable).eq_ignore_ascii_case(exe))
        .map(|game| game.id.clone())
}

/// Estado de los juegos ahora.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Observed {
    /// Juego en primer plano (por id).
    pub foreground: Option<String>,
    /// Juego que sigue abierto (aunque no esté al frente).
    pub running: Option<String>,
}

/// Qué aplicar ahora por los perfiles.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SessionPlan {
    /// Perfil en uso (juego al frente con perfil activo).
    pub active: Option<String>,
    /// Look que reemplaza al del escritorio.
    pub look: Option<Look>,
    /// Crosshair: `Some(true/false)` fuerza; `None` deja el de la pestaña.
    pub crosshair: Option<bool>,
    /// Resolución que tiene que estar aplicada (juego abierto).
    pub resolution: Option<SavedMode>,
}

/// Resuelve qué aplicar con los perfiles y lo que se observa.
pub fn resolve(settings: &ProfileSettings, observed: &Observed) -> SessionPlan {
    let mut plan = SessionPlan::default();
    if !settings.auto_switch {
        return plan;
    }
    let profile = |id: &str| {
        settings
            .profiles
            .iter()
            .find(|profile| profile.enabled && profile.game_id == id)
    };
    if let Some(game) = observed.foreground.as_deref()
        && let Some(profile) = profile(game)
    {
        plan.active = Some(game.to_owned());
        plan.look = profile.look.clone();
        plan.crosshair = match profile.crosshair {
            CrosshairChoice::Keep => None,
            CrosshairChoice::On => Some(true),
            CrosshairChoice::Off => Some(false),
        };
    }
    if let Some(game) = observed.running.as_deref()
        && let Some(profile) = profile(game)
    {
        plan.resolution = profile.resolution.clone();
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::resolution::{ModeRequest, Scaling};

    fn stretched() -> SavedMode {
        SavedMode {
            display_id: "d-1".into(),
            mode: ModeRequest {
                width: 1440,
                height: 1080,
                refresh_hz: 240,
                scaling: Scaling::Stretched,
            },
        }
    }

    fn settings() -> ProfileSettings {
        let look = Look {
            saturation: 150.0,
            ..Look::default()
        };
        ProfileSettings {
            auto_switch: true,
            profiles: vec![GameProfile {
                game_id: "cs2".into(),
                enabled: true,
                look: Some(look),
                crosshair: CrosshairChoice::Off,
                resolution: Some(stretched()),
            }],
            custom_games: vec![CustomGame {
                id: "custom:minecraft.exe".into(),
                name: "Minecraft".into(),
                executable: "Minecraft.exe".into(),
            }],
        }
    }

    fn observed(foreground: Option<&str>, running: Option<&str>) -> Observed {
        Observed {
            foreground: foreground.map(str::to_owned),
            running: running.map(str::to_owned),
        }
    }

    #[test]
    fn games_match_without_case_or_folder() {
        let custom = settings().custom_games;
        assert_eq!(match_game("CS2.EXE", &custom).as_deref(), Some("cs2"));
        assert_eq!(
            match_game(r"C:\Games\Apex\r5apex_dx12.exe", &custom).as_deref(),
            Some("apex")
        );
        assert_eq!(
            match_game("minecraft.exe", &custom).as_deref(),
            Some("custom:minecraft.exe")
        );
        assert_eq!(
            match_game("RiotClientServices.exe", &custom),
            None,
            "el launcher no"
        );
        assert_eq!(match_game("cs2.exe.bak", &custom), None);
    }

    #[test]
    fn entering_the_game_applies_everything() {
        let plan = resolve(&settings(), &observed(Some("cs2"), Some("cs2")));
        assert_eq!(plan.active.as_deref(), Some("cs2"));
        assert_eq!(plan.look.map(|look| look.saturation), Some(150.0));
        assert_eq!(plan.crosshair, Some(false));
        assert_eq!(plan.resolution, Some(stretched()));
    }

    #[test]
    fn alt_tab_restores_the_desktop_look_but_keeps_the_resolution() {
        let plan = resolve(&settings(), &observed(None, Some("cs2")));
        assert_eq!(plan.active, None);
        assert_eq!(plan.look, None, "vuelve el look del escritorio");
        assert_eq!(plan.crosshair, None);
        assert_eq!(
            plan.resolution,
            Some(stretched()),
            "la resolución sigue hasta cerrar el juego"
        );
    }

    #[test]
    fn closing_the_game_reverts_the_resolution() {
        let plan = resolve(&settings(), &observed(None, None));
        assert_eq!(plan, SessionPlan::default());
    }

    #[test]
    fn games_without_a_profile_or_disabled_change_nothing() {
        assert_eq!(
            resolve(&settings(), &observed(Some("valorant"), Some("valorant"))),
            SessionPlan::default()
        );
        let mut disabled = settings();
        disabled.profiles[0].enabled = false;
        assert_eq!(
            resolve(&disabled, &observed(Some("cs2"), Some("cs2"))),
            SessionPlan::default()
        );
    }

    #[test]
    fn auto_switch_off_changes_nothing() {
        let mut off = settings();
        off.auto_switch = false;
        assert_eq!(
            resolve(&off, &observed(Some("cs2"), Some("cs2"))),
            SessionPlan::default()
        );
    }

    #[test]
    fn known_games_have_unique_ids_and_executables() {
        let mut exes: Vec<String> = KNOWN_GAMES
            .iter()
            .flat_map(|game| game.executables.iter().map(|exe| exe.to_lowercase()))
            .collect();
        let total = exes.len();
        exes.sort();
        exes.dedup();
        assert_eq!(
            exes.len(),
            total,
            "un ejecutable no puede ser de dos juegos"
        );
        for (id, _) in crate::looks::GAMES {
            assert!(
                KNOWN_GAMES.iter().any(|game| game.id == *id),
                "{id} sin ejecutables"
            );
        }
    }
}
