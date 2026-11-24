//! Ajustes del usuario (`settings.json`, en `%APPDATA%`).
//!
//! La interfaz recibe y envía el objeto completo. Al guardar, la app lo pasa
//! por [`Settings::merged_from_ui`]: la interfaz puede quitar modos
//! confirmados, pero no inventarlos (el atajo solo alterna a modos que el
//! usuario ya conservó una vez en esa pantalla).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::crosshair::Crosshair;
use crate::keys::{Hotkey, check_hotkey};
use crate::model::look::Look;
use crate::model::resolution::ModeRequest;
use crate::profiles::ProfileSettings;

/// Versión del esquema de `settings.json`.
pub const SETTINGS_SCHEMA: u32 = 1;
/// Cuenta regresiva por defecto al cambiar de resolución, en segundos.
pub const DEFAULT_CONFIRM_SECONDS: u32 = 15;
/// Mínimo y máximo de la cuenta regresiva.
pub const CONFIRM_SECONDS_RANGE: (u32, u32) = (5, 60);
/// Modos confirmados que se recuerdan.
pub const MAX_CONFIRMED: usize = 24;
/// Crosshairs guardados como máximo.
pub const MAX_SAVED_CROSSHAIRS: usize = 50;
/// Largo máximo del nombre de un crosshair o look guardado.
pub const MAX_NAME_CHARS: usize = 40;
/// Looks guardados como máximo.
pub const MAX_SAVED_LOOKS: usize = 200;

/// Un modo de una pantalla.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SavedMode {
    pub display_id: String,
    pub mode: ModeRequest,
}

/// Atajos configurables. El de emergencia (`Ctrl + Alt + Shift + R`) es fijo.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct HotkeySettings {
    /// Alterna entre la resolución de Windows y la estirada.
    pub resolution_toggle: Option<Hotkey>,
}

/// Ajustes de la pestaña Resolución.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ResolutionSettings {
    /// Segundos para conservar un modo nuevo antes de que vuelva solo.
    pub confirm_seconds: u32,
    /// Tonos al alternar con el atajo.
    pub tones: bool,
    /// Modo al que alterna el atajo (el último que conservaste).
    pub toggle: Option<SavedMode>,
    /// Modos que conservaste alguna vez: el atajo solo usa estos.
    pub confirmed: Vec<SavedMode>,
}

impl Default for ResolutionSettings {
    fn default() -> Self {
        Self {
            confirm_seconds: DEFAULT_CONFIRM_SECONDS,
            tones: true,
            toggle: None,
            confirmed: Vec::new(),
        }
    }
}

/// Un crosshair guardado por el usuario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SavedCrosshair {
    pub name: String,
    pub crosshair: Crosshair,
}

/// Ajustes del crosshair.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct CrosshairSettings {
    pub enabled: bool,
    /// Pantalla donde se dibuja (`None`: la principal).
    pub display_id: Option<String>,
    /// Que no aparezca en capturas ni transmisiones (Windows 10 2004+).
    pub hide_from_capture: bool,
    /// El crosshair en uso.
    pub current: Crosshair,
    /// Tus crosshairs guardados.
    pub saved: Vec<SavedCrosshair>,
}

/// Un look guardado por el usuario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SavedLook {
    /// Identificador único (lo genera la interfaz).
    pub id: String,
    pub name: String,
    /// Juego para el que se guardó (`None`: cualquiera).
    pub game: Option<String>,
    pub favorite: bool,
    pub look: Look,
}

/// Biblioteca local: todo queda en tu PC.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct LibrarySettings {
    pub looks: Vec<SavedLook>,
}

/// `settings.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct Settings {
    pub schema_version: u32,
    pub hotkeys: HotkeySettings,
    pub resolution: ResolutionSettings,
    pub crosshair: CrosshairSettings,
    pub library: LibrarySettings,
    pub profiles: ProfileSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA,
            hotkeys: HotkeySettings::default(),
            resolution: ResolutionSettings::default(),
            crosshair: CrosshairSettings::default(),
            library: LibrarySettings::default(),
            profiles: ProfileSettings::default(),
        }
    }
}

impl Settings {
    /// Corrige valores fuera de rango o inconsistentes (archivo editado a mano,
    /// versión anterior).
    pub fn sanitized(mut self) -> Self {
        self.schema_version = SETTINGS_SCHEMA;
        let (min, max) = CONFIRM_SECONDS_RANGE;
        self.resolution.confirm_seconds = self.resolution.confirm_seconds.clamp(min, max);

        let mut unique = Vec::with_capacity(self.resolution.confirmed.len());
        for saved in self.resolution.confirmed.drain(..) {
            if !unique.contains(&saved) {
                unique.push(saved);
            }
        }
        unique.truncate(MAX_CONFIRMED);
        self.resolution.confirmed = unique;

        if let Some(toggle) = &self.resolution.toggle
            && !self.resolution.confirmed.contains(toggle)
        {
            self.resolution.toggle = None;
        }
        if let Some(hotkey) = &self.hotkeys.resolution_toggle
            && check_hotkey(hotkey).error.is_some()
        {
            self.hotkeys.resolution_toggle = None;
        }

        self.crosshair.current = self.crosshair.current.clamped();
        self.crosshair.saved.truncate(MAX_SAVED_CROSSHAIRS);
        for saved in &mut self.crosshair.saved {
            saved.crosshair = saved.crosshair.clamped();
            let name: String = saved.name.trim().chars().take(MAX_NAME_CHARS).collect();
            saved.name = name;
        }
        self.crosshair.saved.retain(|saved| !saved.name.is_empty());

        let mut ids = std::collections::BTreeSet::new();
        self.library.looks.retain(|saved| {
            !saved.id.trim().is_empty()
                && !saved.name.trim().is_empty()
                && ids.insert(saved.id.clone())
        });
        self.library.looks.truncate(MAX_SAVED_LOOKS);
        for saved in &mut self.library.looks {
            saved.look = saved.look.clamped();
            saved.name = saved.name.trim().chars().take(MAX_NAME_CHARS).collect();
        }

        // Un perfil por juego, y su resolución tiene que estar conservada.
        let mut games = std::collections::BTreeSet::new();
        self.profiles
            .profiles
            .retain(|profile| games.insert(profile.game_id.clone()));
        let confirmed = self.resolution.confirmed.clone();
        for profile in &mut self.profiles.profiles {
            if let Some(look) = &mut profile.look {
                *look = look.clamped();
            }
            if profile
                .resolution
                .as_ref()
                .is_some_and(|mode| !confirmed.contains(mode))
            {
                profile.resolution = None;
            }
        }
        let mut exes = std::collections::BTreeSet::new();
        self.profiles.custom_games.retain(|game| {
            !game.executable.trim().is_empty() && exes.insert(game.executable.to_lowercase())
        });
        self.profiles.custom_games.truncate(100);
        self
    }

    /// El usuario conservó un modo: queda confirmado y pasa a ser el destino
    /// del atajo.
    pub fn remember_kept(&mut self, display_id: &str, mode: ModeRequest) {
        let saved = SavedMode {
            display_id: display_id.to_owned(),
            mode,
        };
        self.resolution.confirmed.retain(|item| item != &saved);
        self.resolution.confirmed.insert(0, saved.clone());
        self.resolution.confirmed.truncate(MAX_CONFIRMED);
        self.resolution.toggle = Some(saved);
    }

    /// `true` si ese modo ya se conservó alguna vez en esa pantalla.
    pub fn is_confirmed(&self, saved: &SavedMode) -> bool {
        self.resolution.confirmed.contains(saved)
    }

    /// Ajustes que mandó la interfaz, sin dejarla agregar modos confirmados.
    pub fn merged_from_ui(&self, incoming: Settings) -> Settings {
        let mut merged = incoming;
        let kept: Vec<SavedMode> = merged
            .resolution
            .confirmed
            .iter()
            .filter(|saved| self.is_confirmed(saved))
            .cloned()
            .collect();
        merged.resolution.confirmed = kept;
        merged.sanitized()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::resolution::Scaling;

    fn mode(width: u32) -> ModeRequest {
        ModeRequest {
            width,
            height: 1080,
            refresh_hz: 200,
            scaling: Scaling::Stretched,
        }
    }

    #[test]
    fn defaults_fill_missing_fields() {
        let settings: Settings = serde_json::from_str("{}").expect("válido");
        assert_eq!(settings.resolution.confirm_seconds, DEFAULT_CONFIRM_SECONDS);
        assert!(settings.resolution.tones);
        assert_eq!(settings.hotkeys.resolution_toggle, None);
    }

    #[test]
    fn sanitize_clamps_and_drops_inconsistencies() {
        let mut settings = Settings::default();
        settings.resolution.confirm_seconds = 999;
        settings.resolution.toggle = Some(SavedMode {
            display_id: "d-1".into(),
            mode: mode(1440),
        });
        settings.hotkeys.resolution_toggle = Some(Hotkey {
            code: "F12".into(),
            ctrl: false,
            alt: false,
            shift: false,
            win: false,
        });
        let clean = settings.sanitized();
        assert_eq!(clean.resolution.confirm_seconds, 60);
        assert_eq!(clean.resolution.toggle, None, "no estaba confirmado");
        assert_eq!(clean.hotkeys.resolution_toggle, None, "F12 está reservada");
    }

    #[test]
    fn kept_modes_become_the_toggle_target() {
        let mut settings = Settings::default();
        settings.remember_kept("d-1", mode(1440));
        settings.remember_kept("d-1", mode(1280));
        settings.remember_kept("d-1", mode(1440));
        assert_eq!(settings.resolution.confirmed.len(), 2, "sin repetidos");
        assert_eq!(settings.resolution.confirmed[0].mode.width, 1440);
        assert_eq!(
            settings.resolution.toggle.as_ref().map(|t| t.mode.width),
            Some(1440)
        );
    }

    #[test]
    fn the_interface_cannot_invent_confirmed_modes() {
        let mut current = Settings::default();
        current.remember_kept("d-1", mode(1440));

        let mut incoming = current.clone();
        let invented = SavedMode {
            display_id: "d-1".into(),
            mode: mode(1600),
        };
        incoming.resolution.confirmed.push(invented.clone());
        incoming.resolution.toggle = Some(invented);
        incoming.resolution.tones = false;

        let merged = current.merged_from_ui(incoming);
        assert_eq!(merged.resolution.confirmed.len(), 1);
        assert_eq!(merged.resolution.toggle, None);
        assert!(!merged.resolution.tones, "el resto sí se acepta");
    }
}
