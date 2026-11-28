//! Comandos del preset «máximo FPS»: cambia opciones del archivo de ajustes del
//! juego (el mismo que escribe su menú de gráficos), siempre con el juego
//! cerrado, y guarda los valores del jugador para poder restaurarlos.

use vres_core::game_tune::GameTuneStatus;
use vres_core::ipc::{ErrorCode, IpcError};

/// Juegos con preset de FPS (por ahora, el mismo de los gráficos potato).
fn game(game_id: &str) -> Option<vres_core::potato::PotatoGame> {
    vres_core::potato::games()
        .into_iter()
        .find(|game| game.id == game_id && vres_core::game_tune::preset(game_id).is_some())
}

#[cfg(windows)]
mod imp {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};

    use vres_core::game_tune::{self, GameTuneStatus, TuneSetting};
    use vres_core::ipc::{ErrorCode, IpcError};
    use vres_core::potato::PotatoGame;

    use super::super::potato_file::{settings_files, write_atomic};

    /// Valores del jugador antes del preset: archivo → clave → valor JSON.
    type Originals = BTreeMap<String, BTreeMap<String, String>>;

    fn originals_path() -> Option<PathBuf> {
        vres_engine::state_dir().map(|dir| dir.join("game-tune-originals.json"))
    }

    fn load(path: &Path) -> Originals {
        fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    fn save(path: &Path, originals: &Originals) -> Result<(), String> {
        if originals.is_empty() {
            return match fs::remove_file(path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error.to_string()),
            };
        }
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|error| error.to_string())?;
        }
        let json = serde_json::to_string_pretty(originals).map_err(|error| error.to_string())?;
        write_atomic(path, &json).map_err(|error| error.to_string())
    }

    fn running(game: &PotatoGame) -> bool {
        vres_win::process::process_names()
            .values()
            .any(|exe| exe.eq_ignore_ascii_case(&game.executable))
    }

    pub fn status(game: &PotatoGame) -> GameTuneStatus {
        let preset = game_tune::preset(&game.id).unwrap_or(&[]);
        let files = settings_files(&game.id);
        let settings = files
            .first()
            .and_then(|file| fs::read_to_string(file).ok())
            .map(|text| game_tune::settings(&text, preset))
            .unwrap_or_default();
        let applied = originals_path().is_some_and(|path| {
            let originals = load(&path);
            files
                .iter()
                .any(|file| originals.contains_key(&file.to_string_lossy().into_owned()))
        });
        GameTuneStatus {
            game_id: game.id.clone(),
            game_name: game.name.clone(),
            found: !files.is_empty(),
            game_running: running(game),
            applied,
            settings,
        }
    }

    fn check(game: &PotatoGame) -> Result<(Vec<PathBuf>, PathBuf), IpcError> {
        if running(game) {
            return Err(IpcError::new(ErrorCode::GameRunning, "game is running"));
        }
        let files = settings_files(&game.id);
        if files.is_empty() {
            return Err(IpcError::new(
                ErrorCode::GameTuneNotFound,
                "no settings file",
            ));
        }
        let originals = originals_path()
            .ok_or_else(|| IpcError::new(ErrorCode::GameTuneFailed, "LOCALAPPDATA is not set"))?;
        Ok((files, originals))
    }

    pub fn apply(game: &PotatoGame) -> Result<(), IpcError> {
        let (files, originals_file) = check(game)?;
        let preset = game_tune::preset(&game.id).unwrap_or(&[]);
        apply_to(&files, &originals_file, preset)
            .map_err(|error| IpcError::new(ErrorCode::GameTuneFailed, error))
    }

    pub fn restore(game: &PotatoGame) -> Result<(), IpcError> {
        let (files, originals_file) = check(game)?;
        restore_to(&files, &originals_file)
            .map_err(|error| IpcError::new(ErrorCode::GameTuneFailed, error))
    }

    pub(super) fn apply_to(
        files: &[PathBuf],
        originals_file: &Path,
        preset: &[TuneSetting],
    ) -> Result<(), String> {
        let mut originals = load(originals_file);
        for file in files {
            let name = file.to_string_lossy().into_owned();
            let mut text = fs::read_to_string(file).map_err(|error| format!("{name}: {error}"))?;
            let saved = originals.entry(name.clone()).or_default();
            let mut changed = false;
            for setting in preset {
                let Some(current) = game_tune::read_value(&text, setting.key) else {
                    continue;
                };
                if current == setting.value {
                    continue;
                }
                // Lo primero que había es lo del jugador: no se pisa al reaplicar.
                saved
                    .entry(setting.key.to_string())
                    .or_insert_with(|| current.to_string());
                text = game_tune::write_value(&text, setting.key, setting.value)
                    .ok_or("setting vanished")?;
                changed = true;
            }
            if saved.is_empty() {
                originals.remove(&name);
            }
            if changed {
                write_atomic(file, &text).map_err(|error| format!("{name}: {error}"))?;
            }
        }
        save(originals_file, &originals)
    }

    pub(super) fn restore_to(files: &[PathBuf], originals_file: &Path) -> Result<(), String> {
        let mut originals = load(originals_file);
        for file in files {
            let name = file.to_string_lossy().into_owned();
            let Some(saved) = originals.remove(&name) else {
                continue;
            };
            let mut text = fs::read_to_string(file).map_err(|error| format!("{name}: {error}"))?;
            for (key, value) in &saved {
                if let Some(edited) = game_tune::write_value(&text, key, value) {
                    text = edited;
                }
            }
            write_atomic(file, &text).map_err(|error| format!("{name}: {error}"))?;
        }
        save(originals_file, &originals)
    }
}

/// Estado del preset de FPS de cada juego (solo lee).
#[tauri::command]
pub async fn game_tune_status() -> Result<Vec<GameTuneStatus>, IpcError> {
    tauri::async_runtime::spawn_blocking(|| {
        #[cfg(windows)]
        {
            vres_core::potato::games()
                .iter()
                .filter(|game| vres_core::game_tune::preset(&game.id).is_some())
                .map(imp::status)
                .collect()
        }
        #[cfg(not(windows))]
        {
            Vec::new()
        }
    })
    .await
    .map_err(|error| IpcError::new(ErrorCode::Internal, error.to_string()))
}

/// Aplica el preset (`restore = false`) o devuelve los ajustes del jugador.
#[tauri::command]
pub async fn game_tune_apply(game: String, restore: bool) -> Result<GameTuneStatus, IpcError> {
    tauri::async_runtime::spawn_blocking(move || {
        let Some(target) = self::game(&game) else {
            return Err(IpcError::new(ErrorCode::GameTuneNotFound, "unknown game"));
        };
        #[cfg(windows)]
        {
            if restore {
                imp::restore(&target)?;
            } else {
                imp::apply(&target)?;
            }
            Ok(imp::status(&target))
        }
        #[cfg(not(windows))]
        {
            let _ = (target, restore);
            Err(IpcError::new(ErrorCode::GameTuneFailed, "windows only"))
        }
    })
    .await
    .map_err(|error| IpcError::new(ErrorCode::Internal, error.to_string()))?
}

#[cfg(all(test, windows))]
mod tests {
    use std::fs;

    use vres_core::game_tune::{ROBRUN_MAX_FPS, read_value};

    use super::imp::{apply_to, restore_to};

    #[test]
    fn apply_then_restore_gives_back_the_player_file() {
        let dir = std::env::temp_dir().join(format!("vres-game-tune-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings_steam_1_2.dat");
        let originals = dir.join("state").join("game-tune-originals.json");
        let before = r#"{"setting_quality":"high","render_scale_setting":0.800000012,"hbao_setting":true,"settings_grass":"high","depth_of_field":false}"#;
        fs::write(&file, before).unwrap();
        let files = vec![file.clone()];

        apply_to(&files, &originals, &ROBRUN_MAX_FPS).unwrap();
        let tuned = fs::read_to_string(&file).unwrap();
        assert_eq!(read_value(&tuned, "setting_quality"), Some("\"custom\""));
        assert_eq!(read_value(&tuned, "hbao_setting"), Some("false"));
        assert_eq!(read_value(&tuned, "settings_grass"), Some("\"low\""));
        assert!(tuned.contains("0.800000012"));

        // Reaplicar no pisa los valores guardados del jugador.
        apply_to(&files, &originals, &ROBRUN_MAX_FPS).unwrap();
        restore_to(&files, &originals).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), before);
        assert!(!originals.exists());
        fs::remove_dir_all(&dir).unwrap();
    }
}
