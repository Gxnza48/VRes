//! Ajustes de la app: arranque con Windows, atajos y preferencias.

use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt;
use vres_core::ipc::{ErrorCode, IpcError};
use vres_core::keys::{Hotkey, HotkeyCheck, check_hotkey_among};
use vres_core::settings::Settings;
use vres_engine::Command;

use crate::engine_bridge::{EVENT_SETTINGS, EngineState};
use crate::settings::SettingsState;

fn failed(error: impl ToString) -> IpcError {
    IpcError::new(ErrorCode::Internal, error.to_string())
}

/// `true` si VRes arranca con Windows (directo a la bandeja).
#[tauri::command]
pub async fn settings_get_autostart(app: AppHandle) -> Result<bool, IpcError> {
    app.autolaunch().is_enabled().map_err(failed)
}

/// Activa o desactiva el arranque con Windows. Devuelve el estado final.
#[tauri::command]
pub async fn settings_set_autostart(app: AppHandle, enabled: bool) -> Result<bool, IpcError> {
    let autolaunch = app.autolaunch();
    if enabled {
        autolaunch.enable().map_err(failed)?;
    } else {
        autolaunch.disable().map_err(failed)?;
    }
    autolaunch.is_enabled().map_err(failed)
}

/// Ajustes actuales.
#[tauri::command]
pub async fn settings_get(settings: State<'_, SettingsState>) -> Result<Settings, IpcError> {
    Ok(settings.get())
}

/// Guarda los ajustes. Si un atajo **nuevo** lo usa otro programa, no se
/// guarda nada y los anteriores siguen activos. Un atajo que ya estaba (y que
/// otro programa ocupó) no impide guardar lo demás: ya se avisó al arrancar.
#[tauri::command]
pub async fn settings_update(app: AppHandle, settings: Settings) -> Result<Settings, IpcError> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app
            .try_state::<SettingsState>()
            .ok_or_else(|| failed("settings state missing"))?;
        let current = state.get();
        let merged = current.merged_from_ui(settings);
        if let Some(engine) = app.try_state::<EngineState>()
            && engine.available()
        {
            let registered = engine.configure(&merged);
            let resolution_taken = !registered.resolution
                && merged.hotkeys.resolution_toggle != current.hotkeys.resolution_toggle;
            let crosshair_taken = !registered.crosshair
                && merged.hotkeys.crosshair_toggle != current.hotkeys.crosshair_toggle;
            if resolution_taken || crosshair_taken {
                engine.configure(&current);
                let which = if resolution_taken {
                    "resolution"
                } else {
                    "crosshair"
                };
                return Err(IpcError::new(
                    ErrorCode::HotkeyTaken,
                    format!("the {which} hotkey is used by another program"),
                ));
            }
        }
        state
            .set(merged.clone())
            .map_err(|error| IpcError::new(ErrorCode::SettingsFailed, error))?;
        let _ = app.emit(EVENT_SETTINGS, &merged);
        Ok(merged)
    })
    .await
    .map_err(failed)?
}

/// Revisa un atajo antes de guardarlo (teclas reservadas, choques conocidos y
/// los demás atajos de VRes, `others`).
#[tauri::command]
pub async fn hotkeys_check(hotkey: Hotkey, others: Vec<Hotkey>) -> Result<HotkeyCheck, IpcError> {
    Ok(check_hotkey_among(&hotkey, &others))
}

/// Suspende los atajos configurables mientras se graba uno nuevo (si no, al
/// apretarlo se activaría en vez de grabarse).
#[tauri::command]
pub async fn hotkeys_suspend(
    engine: State<'_, EngineState>,
    suspended: bool,
) -> Result<(), IpcError> {
    engine.send(Command::SuspendHotkeys(suspended));
    Ok(())
}
