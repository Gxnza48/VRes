//! `settings.json` (en `%APPDATA%\io.github.gxnza48.vres`): atajos y
//! preferencias del usuario.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use vres_core::model::resolution::ModeRequest;
use vres_core::settings::Settings;
use vres_core::store::{self, Loaded};
use vres_engine::EngineSettings;

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Ruta de `settings.json`.
fn settings_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|dir| {
        PathBuf::from(dir)
            .join(vres_core::APP_IDENTIFIER)
            .join("settings.json")
    })
}

/// Lo que el motor necesita de los ajustes.
pub fn engine_settings(settings: &Settings) -> EngineSettings {
    EngineSettings {
        toggle_hotkey: settings.hotkeys.resolution_toggle.clone(),
        toggle_target: settings.resolution.toggle.clone(),
        tones: settings.resolution.tones,
        confirm_seconds: settings.resolution.confirm_seconds,
        crosshair: settings.crosshair.clone(),
        crosshair_hotkey: settings.hotkeys.crosshair_toggle.clone(),
        profiles: settings.profiles.clone(),
        enhancers: settings.enhancers.clone(),
    }
}

/// Ajustes del usuario, administrados por Tauri.
pub struct SettingsState {
    settings: Mutex<Settings>,
    path: Option<PathBuf>,
}

impl SettingsState {
    /// Lee `settings.json`. Si falta o está dañado, se usan los valores por
    /// defecto (el dañado queda en cuarentena).
    pub fn load() -> Self {
        let path = settings_path();
        let settings = match path.as_ref().map(|path| store::read_json::<Settings>(path)) {
            Some(Loaded::Ok(settings)) => settings.sanitized(),
            Some(Loaded::Corrupt { quarantined }) => {
                tracing::warn!(
                    ?quarantined,
                    "settings.json dañado: se usan los valores por defecto"
                );
                Settings::default()
            }
            _ => Settings::default(),
        };
        Self {
            settings: Mutex::new(settings),
            path,
        }
    }

    /// Ajustes actuales.
    pub fn get(&self) -> Settings {
        lock(&self.settings).clone()
    }

    /// Reemplaza y guarda.
    pub fn set(&self, settings: Settings) -> Result<(), String> {
        *lock(&self.settings) = settings.clone();
        match &self.path {
            Some(path) => {
                store::write_json_atomic(path, &settings).map_err(|error| error.to_string())
            }
            None => Ok(()),
        }
    }

    /// El usuario conservó un modo: queda confirmado y es el destino del atajo.
    pub fn remember_kept(&self, display_id: &str, mode: ModeRequest) -> Result<Settings, String> {
        let mut settings = self.get();
        settings.remember_kept(display_id, mode);
        self.set(settings.clone())?;
        Ok(settings)
    }
}
