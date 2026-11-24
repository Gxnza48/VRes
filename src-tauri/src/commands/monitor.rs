//! Comandos de la pestaña Monitor: controles por hardware (DDC/CI en monitores
//! externos, brillo del panel por WMI). Cada llamada habla con el monitor y
//! puede tardar: corren en un hilo aparte y de a una por vez. VRes guarda los
//! valores de antes de tocarlos para volver a ellos con un botón.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use tauri::State;
use vres_core::ipc::{ErrorCode, IpcError};
use vres_core::model::display::DisplayInfo;
use vres_core::monitor::{DdcControl, HardwareInitial, MonitorDetails, VcpValue};
use vres_core::store::{self, Loaded};
use vres_win::{ddc, panel};

use crate::engine_bridge::EngineState;

/// De a una llamada por vez: dos conversaciones I²C a la vez con el mismo
/// monitor se mezclan.
static HARDWARE: Mutex<()> = Mutex::new(());

fn initial_path() -> Option<PathBuf> {
    vres_engine::state_dir().map(|dir| dir.join("hardware-initial.json"))
}

fn load_initial() -> HardwareInitial {
    match initial_path().map(|path| store::read_json::<HardwareInitial>(&path)) {
        Some(Loaded::Ok(initial)) => initial,
        _ => HardwareInitial {
            schema_version: 1,
            ..HardwareInitial::default()
        },
    }
}

fn save_initial(initial: &HardwareInitial) -> Result<(), IpcError> {
    let path = initial_path().ok_or_else(|| failed("no state folder"))?;
    store::write_json_atomic(&path, initial).map_err(failed)
}

fn failed(error: impl ToString) -> IpcError {
    IpcError::new(ErrorCode::HardwareFailed, error.to_string())
}

fn unsupported() -> IpcError {
    IpcError::new(
        ErrorCode::HardwareUnsupported,
        "the display does not support this control",
    )
}

fn find(engine: &EngineState, display_id: &str) -> Result<DisplayInfo, IpcError> {
    engine
        .displays()
        .into_iter()
        .find(|display| display.id == display_id)
        .ok_or_else(|| IpcError::new(ErrorCode::DisplayGone, "display not found"))
}

/// Corre `work` en un hilo aparte, sin otra llamada de hardware a la vez.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, IpcError> + Send + 'static,
) -> Result<T, IpcError> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = HARDWARE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        work()
    })
    .await
    .map_err(|error| IpcError::new(ErrorCode::Internal, error.to_string()))?
}

/// Lo que se puede ajustar por hardware en cada pantalla.
#[tauri::command]
pub async fn monitor_details(
    engine: State<'_, EngineState>,
) -> Result<Vec<MonitorDetails>, IpcError> {
    let displays = engine.displays();
    blocking(move || {
        let initial = load_initial();
        Ok(displays
            .iter()
            .map(|display| MonitorDetails {
                display_id: display.id.clone(),
                scale_percent: ddc::scale_percent(&display.gdi_name),
                ddc: if display.internal {
                    BTreeMap::new()
                } else {
                    ddc::read_all(&display.gdi_name).unwrap_or_default()
                },
                panel_brightness: if display.internal {
                    panel::brightness().ok().flatten()
                } else {
                    None
                },
                has_initial: initial.has(&display.id),
            })
            .collect())
    })
    .await
}

/// Cambia un control DDC/CI. Devuelve el valor que quedó (releído).
#[tauri::command]
pub async fn ddc_set(
    engine: State<'_, EngineState>,
    display_id: String,
    control: DdcControl,
    value: u32,
) -> Result<VcpValue, IpcError> {
    let display = find(&engine, &display_id)?;
    blocking(move || {
        let current = ddc::get(&display.gdi_name, control)
            .map_err(failed)?
            .ok_or_else(unsupported)?;
        let mut initial = load_initial();
        if initial.remember_ddc(&display.id, control, current.current) {
            save_initial(&initial)?;
        }
        ddc::set(&display.gdi_name, control, value.min(current.max)).map_err(failed)
    })
    .await
}

/// Cambia el brillo del panel de la notebook (0–100).
#[tauri::command]
pub async fn panel_brightness_set(
    engine: State<'_, EngineState>,
    display_id: String,
    value: u32,
) -> Result<u32, IpcError> {
    let display = find(&engine, &display_id)?;
    if !display.internal {
        return Err(unsupported());
    }
    blocking(move || {
        let current = panel::brightness()
            .map_err(failed)?
            .ok_or_else(unsupported)?;
        let mut initial = load_initial();
        if initial.remember_panel(&display.id, current) {
            save_initial(&initial)?;
        }
        let value = value.min(100);
        panel::set_brightness(value).map_err(failed)?;
        Ok(value)
    })
    .await
}

/// Vuelve a los valores de antes de VRes (brillo, contraste, ganancias).
#[tauri::command]
pub async fn hardware_restore_initial(
    engine: State<'_, EngineState>,
    display_id: String,
) -> Result<(), IpcError> {
    let display = find(&engine, &display_id)?;
    blocking(move || {
        let mut initial = load_initial();
        let Some(values) = initial.take(&display.id) else {
            return Ok(());
        };
        for (control, value) in values.ddc {
            ddc::set(&display.gdi_name, control, value).map_err(failed)?;
        }
        if let Some(brightness) = values.panel_brightness {
            panel::set_brightness(brightness).map_err(failed)?;
        }
        save_initial(&initial)
    })
    .await
}
