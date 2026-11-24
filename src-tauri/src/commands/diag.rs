//! Reporte de diagnóstico (para pegar en un reporte de error) y "restablecer
//! todo". El reporte no incluye el nombre de usuario, la carpeta de usuario
//! ni el nombre del equipo, y nunca se envía solo: el usuario lo copia.

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, State};
use vres_core::ipc::{ErrorCode, IpcError};
use vres_core::monitor::anonymize;
use vres_core::planner::Requested;
use vres_core::settings::Settings;
use vres_engine::ApplyMode;

use crate::engine_bridge::{EVENT_REQUESTED, EVENT_SETTINGS, EngineState};
use crate::settings::SettingsState;

/// Cuántas líneas del log entran en el reporte.
const LOG_LINES: usize = 200;

fn internal(error: impl ToString) -> IpcError {
    IpcError::new(ErrorCode::Internal, error.to_string())
}

/// Datos personales a quitar: la carpeta de usuario (también con las barras
/// escapadas del JSON), el usuario y el equipo.
fn personal_data() -> Vec<(String, &'static str)> {
    let mut secrets = Vec::new();
    if let Ok(profile) = std::env::var("USERPROFILE") {
        secrets.push((profile.replace('\\', "\\\\"), "%USERPROFILE%"));
        secrets.push((profile, "%USERPROFILE%"));
    }
    for (variable, placeholder) in [("USERNAME", "<user>"), ("COMPUTERNAME", "<computer>")] {
        if let Ok(value) = std::env::var(variable) {
            secrets.push((value, placeholder));
        }
    }
    secrets
}

/// Las últimas líneas de un texto.
fn tail(text: &str, lines: usize) -> Vec<&str> {
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].to_vec()
}

/// Reporte de diagnóstico en JSON, sin datos personales.
#[tauri::command]
pub async fn diag_report(
    app: AppHandle,
    engine: State<'_, EngineState>,
    settings: State<'_, SettingsState>,
) -> Result<String, IpcError> {
    let displays = engine.displays();
    let status = engine.status();
    let settings = settings.get();
    let log_dir = app.path().app_log_dir().ok();
    tauri::async_runtime::spawn_blocking(move || {
        let log = log_dir
            .and_then(|dir| std::fs::read_to_string(dir.join("vres.log")).ok())
            .unwrap_or_default();
        let report = json!({
            "app": "VRes",
            "version": vres_core::APP_VERSION,
            "windows": vres_win::system::windows_version(),
            "displays": displays,
            "engine": status,
            "settings": {
                "savedLooks": settings.library.looks.len(),
                "savedCrosshairs": settings.crosshair.saved.len(),
                "confirmedModes": settings.resolution.confirmed.len(),
                "crosshairEnabled": settings.crosshair.enabled,
                "autoSwitch": settings.profiles.auto_switch,
                "profilesEnabled": settings.profiles.profiles.iter().filter(|profile| profile.enabled).count(),
                "customGames": settings.profiles.custom_games.len(),
                "darkBoostArmed": settings.enhancers.dark_boost.armed,
                "zoomArmed": settings.enhancers.zoom.armed,
                "theme": settings.appearance.theme,
                "background": settings.appearance.background.is_some(),
                "closeToTray": settings.behavior.close_to_tray,
            },
            "log": tail(&log, LOG_LINES),
        });
        let text = serde_json::to_string_pretty(&report).map_err(internal)?;
        let secrets = personal_data();
        let pairs: Vec<(&str, &str)> = secrets
            .iter()
            .map(|(secret, placeholder)| (secret.as_str(), *placeholder))
            .collect();
        Ok(anonymize(&text, &pairs))
    })
    .await
    .map_err(internal)?
}

/// Vuelve todo a como estaba al instalar: ajustes, biblioteca, perfiles,
/// fondo y el look. No toca el arranque con Windows.
#[tauri::command]
pub async fn settings_reset(app: AppHandle) -> Result<Settings, IpcError> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app
            .try_state::<SettingsState>()
            .ok_or_else(|| internal("settings state missing"))?;
        let defaults = Settings::default();
        if let Some(path) = std::env::var_os("APPDATA").map(|dir| {
            std::path::PathBuf::from(dir)
                .join(vres_core::APP_IDENTIFIER)
                .join("media")
                .join("background.bin")
        }) {
            let _ = std::fs::remove_file(path);
        }
        if let Some(engine) = app.try_state::<EngineState>()
            && engine.available()
        {
            engine.configure(&defaults);
            let requested = engine.update(ApplyMode::Commit, |requested| {
                *requested = Requested::default();
            });
            let _ = app.emit(EVENT_REQUESTED, &requested);
        }
        state
            .set(defaults.clone())
            .map_err(|error| IpcError::new(ErrorCode::SettingsFailed, error))?;
        let _ = app.emit(EVENT_SETTINGS, &defaults);
        Ok(defaults)
    })
    .await
    .map_err(internal)?
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Con los datos reales de la PC donde corre la prueba (también en CI).
    #[test]
    fn the_report_has_no_personal_data_of_this_pc() {
        let profile = std::env::var("USERPROFILE").unwrap_or_default();
        let user = std::env::var("USERNAME").unwrap_or_default();
        let computer = std::env::var("COMPUTERNAME").unwrap_or_default();
        let report = serde_json::to_string_pretty(&json!({
            "log": [format!(r"opened {profile}\AppData\Local\x"), format!("user {user} on {computer}")],
            "path": profile,
        }))
        .expect("json");
        let secrets = personal_data();
        let pairs: Vec<(&str, &str)> = secrets
            .iter()
            .map(|(secret, placeholder)| (secret.as_str(), *placeholder))
            .collect();
        let clean = anonymize(&report, &pairs).to_lowercase();
        for value in [&user, &computer] {
            if value.chars().count() >= 2 {
                assert!(
                    !clean.contains(&value.to_lowercase()),
                    "quedó un dato personal"
                );
            }
        }
        assert!(clean.contains("%userprofile%"));
    }

    #[test]
    fn tail_keeps_the_last_lines() {
        assert_eq!(tail("a\nb\nc", 2), vec!["b", "c"]);
        assert_eq!(tail("a", 5), vec!["a"]);
        assert!(tail("", 3).is_empty());
    }
}
