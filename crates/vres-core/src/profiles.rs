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

/// Los juegos con soporte de fábrica. Los nombres de ejecutable se
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
    KnownGame {
        id: "robrun",
        name: "ROB RUN",
        executables: &["ROB RUN.exe"],
        anti_cheat: "Easy Anti-Cheat",
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
    /// La config principal (look y crosshair) vale solo con un juego al
    /// frente; en el escritorio la pantalla queda normal.
    pub main_only_in_games: bool,
    /// Juegos donde vale la config principal (vacío: cualquier juego).
    pub main_games: Vec<String>,
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
    if settings.main_only_in_games {
        let in_main_game = observed.foreground.as_deref().is_some_and(|game| {
            settings.main_games.is_empty() || settings.main_games.iter().any(|id| id == game)
        });
        if !in_main_game {
            // Fuera de esos juegos, la pantalla queda como Windows la muestra.
            plan.look = Some(Look::default());
            plan.crosshair = Some(false);
        }
    }
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
        // Sin look ni crosshair propios, el perfil deja lo de la config principal.
        if profile.look.is_some() {
            plan.look = profile.look.clone();
        }
        match profile.crosshair {
            CrosshairChoice::Keep => {}
            CrosshairChoice::On => plan.crosshair = Some(true),
            CrosshairChoice::Off => plan.crosshair = Some(false),
        }
    }
    if let Some(game) = observed.running.as_deref()
        && let Some(profile) = profile(game)
    {
        plan.resolution = profile.resolution.clone();
    }
    plan
}

/// Si el crosshair se ve: manda lo que se cambió con el atajo mientras un
/// perfil lo controla, después el perfil y por último el interruptor de la
/// pestaña Crosshair.
pub fn crosshair_visible(enabled: bool, profile: Option<bool>, hotkey: Option<bool>) -> bool {
    hotkey.or(profile).unwrap_or(enabled)
}

/// Lo que cambia el atajo del crosshair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrosshairToggle {
    /// El interruptor de la pestaña Crosshair (queda guardado).
    Setting(bool),
    /// Solo mientras siga en uso el perfil que lo controla.
    Session(bool),
}

/// El atajo del crosshair invierte lo que se ve. Si un perfil lo controla, el
/// cambio vale para esa partida; si no, cambia el interruptor de la pestaña.
pub fn toggle_crosshair(
    enabled: bool,
    profile: Option<bool>,
    hotkey: Option<bool>,
) -> CrosshairToggle {
    let next = !crosshair_visible(enabled, profile, hotkey);
    if profile.is_some() {
        CrosshairToggle::Session(next)
    } else {
        CrosshairToggle::Setting(next)
    }
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
            ..ProfileSettings::default()
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
    fn the_crosshair_hotkey_flips_the_setting_without_a_profile() {
        assert_eq!(
            toggle_crosshair(false, None, None),
            CrosshairToggle::Setting(true)
        );
        assert_eq!(
            toggle_crosshair(true, None, None),
            CrosshairToggle::Setting(false)
        );
    }

    #[test]
    fn with_a_profile_the_crosshair_hotkey_only_lasts_the_session() {
        // El perfil lo apaga aunque la pestaña lo tenga prendido: el atajo lo muestra.
        assert_eq!(
            toggle_crosshair(true, Some(false), None),
            CrosshairToggle::Session(true)
        );
        assert!(crosshair_visible(true, Some(false), Some(true)));
        // Otra vez: se vuelve a ocultar.
        assert_eq!(
            toggle_crosshair(true, Some(false), Some(true)),
            CrosshairToggle::Session(false)
        );
        // El perfil lo prende aunque la pestaña lo tenga apagado: el atajo lo oculta.
        assert_eq!(
            toggle_crosshair(false, Some(true), None),
            CrosshairToggle::Session(false)
        );
    }

    #[test]
    fn crosshair_visibility_follows_hotkey_then_profile_then_setting() {
        assert!(crosshair_visible(true, None, None));
        assert!(!crosshair_visible(true, Some(false), None));
        assert!(crosshair_visible(false, Some(true), None));
        assert!(!crosshair_visible(false, Some(true), Some(false)));
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

    #[test]
    fn main_config_only_in_games() {
        let mut only = settings();
        only.auto_switch = false;
        only.main_only_in_games = true;
        // En el escritorio: pantalla normal y sin crosshair.
        let desktop = resolve(&only, &observed(None, None));
        assert_eq!(desktop.look, Some(Look::default()));
        assert_eq!(desktop.crosshair, Some(false));
        // Con un juego al frente: la config principal.
        assert_eq!(
            resolve(&only, &observed(Some("cs2"), Some("cs2"))),
            SessionPlan::default()
        );
        // Con una lista de juegos, solo en esos.
        only.main_games = vec!["fortnite".into()];
        let other = resolve(&only, &observed(Some("cs2"), Some("cs2")));
        assert_eq!(other.crosshair, Some(false));
        assert_eq!(
            resolve(&only, &observed(Some("fortnite"), None)),
            SessionPlan::default()
        );
    }
}
