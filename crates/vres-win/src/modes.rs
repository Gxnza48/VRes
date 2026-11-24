//! Modos de pantalla: enumerar, probar, aplicar de forma **temporal** y volver
//! al modo guardado en Windows. También el escalado (CCD), la resolución nativa
//! y el interruptor de HDR.
//!
//! Regla: VRes nunca escribe un modo en el registro (`CDS_UPDATEREGISTRY` ni
//! `SDC_SAVE_TO_DATABASE`). "Revertir" es siempre volver al modo guardado, que
//! sigue intacto.

use vres_core::model::resolution::{DisplayMode, ModeRequest, Scaling, normalize_modes};
use windows::Win32::Devices::Display::{
    DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_PREFERRED_MODE,
    DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_DEVICE_INFO_SET_ADVANCED_COLOR_STATE,
    DISPLAYCONFIG_DEVICE_INFO_TYPE, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO,
    DISPLAYCONFIG_SCALING, DISPLAYCONFIG_SCALING_ASPECTRATIOCENTEREDMAX,
    DISPLAYCONFIG_SCALING_CENTERED, DISPLAYCONFIG_SCALING_CUSTOM, DISPLAYCONFIG_SCALING_IDENTITY,
    DISPLAYCONFIG_SCALING_PREFERRED, DISPLAYCONFIG_SCALING_STRETCHED,
    DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE, DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE_0,
    DISPLAYCONFIG_SOURCE_DEVICE_NAME, DISPLAYCONFIG_TARGET_PREFERRED_MODE,
    DisplayConfigSetDeviceInfo, SDC_ALLOW_CHANGES, SDC_APPLY, SDC_USE_SUPPLIED_DISPLAY_CONFIG,
    SetDisplayConfig,
};
use windows::Win32::Foundation::{ERROR_SUCCESS, LUID, POINTL};
use windows::Win32::Graphics::Gdi::{
    CDS_FULLSCREEN, CDS_TEST, CDS_TYPE, ChangeDisplaySettingsExW, DEVMODE_DISPLAY_FIXED_OUTPUT,
    DEVMODEW, DEVMODEW_0, DEVMODEW_0_1, DISP_CHANGE_SUCCESSFUL, DM_BITSPERPEL,
    DM_DISPLAYFIXEDOUTPUT, DM_DISPLAYFREQUENCY, DM_PELSHEIGHT, DM_PELSWIDTH, DMDFO_CENTER,
    DMDFO_DEFAULT, DMDFO_STRETCH, DMDO_DEFAULT, ENUM_CURRENT_SETTINGS, ENUM_DISPLAY_SETTINGS_FLAGS,
    ENUM_DISPLAY_SETTINGS_MODE, ENUM_REGISTRY_SETTINGS, EnumDisplaySettingsExW,
};
use windows::core::HSTRING;

use crate::displays::{device_info, query_active_paths, utf16_to_string};
use crate::{WinError, WinResult};

/// `DISPLAYCONFIG_DEVICE_INFO_SET_HDR_STATE` (Windows 11 24H2+). No está en el
/// crate `windows`: se declara a mano.
const SET_HDR_STATE: i32 = 16;
/// `DM_INTERLACED` en `dmDisplayFlags`.
const DM_INTERLACED: u32 = 0x2;

#[repr(C)]
struct SetHdrState {
    header: DISPLAYCONFIG_DEVICE_INFO_HEADER,
    /// Bit 0: `enableHdr`.
    value: u32,
}

fn devmode() -> DEVMODEW {
    DEVMODEW {
        dmSize: size_of::<DEVMODEW>() as u16,
        ..Default::default()
    }
}

fn read_settings(gdi_name: &str, index: ENUM_DISPLAY_SETTINGS_MODE) -> Option<DEVMODEW> {
    let name = HSTRING::from(gdi_name);
    let mut dm = devmode();
    // SAFETY: `dm` tiene `dmSize` correcto y vive durante la llamada; `name`
    // es una cadena terminada en cero.
    unsafe { EnumDisplaySettingsExW(&name, index, &mut dm, ENUM_DISPLAY_SETTINGS_FLAGS(0)) }
        .as_bool()
        .then_some(dm)
}

fn to_mode(dm: &DEVMODEW) -> DisplayMode {
    DisplayMode::new(dm.dmPelsWidth, dm.dmPelsHeight, dm.dmDisplayFrequency)
}

/// Modos que el monitor admite (32 bits, sin entrelazado), del más grande al
/// más chico y sin repetidos.
pub fn list_modes(gdi_name: &str) -> Vec<DisplayMode> {
    let mut modes = Vec::new();
    for index in 0..4096 {
        let Some(dm) = read_settings(gdi_name, ENUM_DISPLAY_SETTINGS_MODE(index)) else {
            break;
        };
        // SAFETY: para pantallas, la unión contiene `dmDisplayFlags`.
        let interlaced = unsafe { dm.Anonymous2.dmDisplayFlags } & DM_INTERLACED != 0;
        // Frecuencia 0 o 1 significa "la del hardware": no sirve para elegir.
        if dm.dmBitsPerPel == 32 && dm.dmDisplayFrequency > 1 && !interlaced {
            modes.push(to_mode(&dm));
        }
    }
    normalize_modes(modes)
}

/// Modo actual.
pub fn current_mode(gdi_name: &str) -> WinResult<DisplayMode> {
    read_settings(gdi_name, ENUM_CURRENT_SETTINGS)
        .map(|dm| to_mode(&dm))
        .ok_or_else(|| WinError::last("EnumDisplaySettingsExW"))
}

/// Modo guardado en Windows: al que vuelve VRes.
pub fn saved_mode(gdi_name: &str) -> WinResult<DisplayMode> {
    read_settings(gdi_name, ENUM_REGISTRY_SETTINGS)
        .map(|dm| to_mode(&dm))
        .ok_or_else(|| WinError::last("EnumDisplaySettingsExW"))
}

fn fixed_output(scaling: Scaling) -> DEVMODE_DISPLAY_FIXED_OUTPUT {
    match scaling {
        Scaling::Stretched => DMDFO_STRETCH,
        Scaling::Centered => DMDFO_CENTER,
        _ => DMDFO_DEFAULT,
    }
}

fn devmode_for(request: &ModeRequest) -> DEVMODEW {
    let mut dm = devmode();
    dm.dmPelsWidth = request.width;
    dm.dmPelsHeight = request.height;
    dm.dmDisplayFrequency = request.refresh_hz;
    dm.dmBitsPerPel = 32;
    dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY | DM_BITSPERPEL;
    if matches!(request.scaling, Scaling::Stretched | Scaling::Centered) {
        dm.Anonymous1 = DEVMODEW_0 {
            Anonymous2: DEVMODEW_0_1 {
                dmPosition: POINTL::default(),
                dmDisplayOrientation: DMDO_DEFAULT,
                dmDisplayFixedOutput: fixed_output(request.scaling),
            },
        };
        dm.dmFields |= DM_DISPLAYFIXEDOUTPUT;
    }
    dm
}

fn change_settings(gdi_name: &str, dm: Option<&DEVMODEW>, flags: CDS_TYPE) -> WinResult<()> {
    let name = HSTRING::from(gdi_name);
    // SAFETY: `dm` (si hay) es un DEVMODEW válido con `dmSize` correcto; sin
    // ventana ni parámetro extra.
    let result = unsafe {
        ChangeDisplaySettingsExW(&name, dm.map(|dm| dm as *const DEVMODEW), None, flags, None)
    };
    if result == DISP_CHANGE_SUCCESSFUL {
        Ok(())
    } else {
        Err(WinError::DisplayChange {
            api: "ChangeDisplaySettingsExW",
            code: result.0,
        })
    }
}

/// Pregunta si el modo se puede aplicar, sin cambiar nada (`CDS_TEST`).
pub fn test_mode(gdi_name: &str, request: &ModeRequest) -> WinResult<()> {
    change_settings(gdi_name, Some(&devmode_for(request)), CDS_TEST)
}

/// Aplica el modo **solo hasta que se revierta** (`CDS_FULLSCREEN`, sin tocar
/// el registro).
pub fn apply_temporary(gdi_name: &str, request: &ModeRequest) -> WinResult<()> {
    change_settings(gdi_name, Some(&devmode_for(request)), CDS_FULLSCREEN)
}

/// Vuelve al modo guardado en Windows.
pub fn revert_to_saved(gdi_name: &str) -> WinResult<()> {
    change_settings(gdi_name, None, CDS_TYPE(0))
}

/// La ruta de CCD de una pantalla, con toda la configuración activa.
fn find_path(
    gdi_name: &str,
) -> WinResult<(
    Vec<DISPLAYCONFIG_PATH_INFO>,
    Vec<DISPLAYCONFIG_MODE_INFO>,
    usize,
)> {
    let (paths, modes) = query_active_paths()?;
    let index = paths
        .iter()
        .position(|path| {
            let mut name = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
            device_info(
                &mut name,
                DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
                path.sourceInfo.adapterId,
                path.sourceInfo.id,
            ) && utf16_to_string(&name.viewGdiDeviceName).eq_ignore_ascii_case(gdi_name)
        })
        .ok_or(WinError::Unsupported("display not found"))?;
    Ok((paths, modes, index))
}

fn from_ccd(scaling: DISPLAYCONFIG_SCALING) -> Scaling {
    match scaling {
        DISPLAYCONFIG_SCALING_IDENTITY => Scaling::Identity,
        DISPLAYCONFIG_SCALING_CENTERED => Scaling::Centered,
        DISPLAYCONFIG_SCALING_STRETCHED => Scaling::Stretched,
        DISPLAYCONFIG_SCALING_ASPECTRATIOCENTEREDMAX => Scaling::AspectRatio,
        DISPLAYCONFIG_SCALING_CUSTOM => Scaling::Custom,
        _ => Scaling::Preferred,
    }
}

fn to_ccd(scaling: Scaling) -> DISPLAYCONFIG_SCALING {
    match scaling {
        Scaling::Identity => DISPLAYCONFIG_SCALING_IDENTITY,
        Scaling::Centered => DISPLAYCONFIG_SCALING_CENTERED,
        Scaling::Stretched => DISPLAYCONFIG_SCALING_STRETCHED,
        Scaling::AspectRatio => DISPLAYCONFIG_SCALING_ASPECTRATIOCENTEREDMAX,
        Scaling::Custom => DISPLAYCONFIG_SCALING_CUSTOM,
        Scaling::Preferred => DISPLAYCONFIG_SCALING_PREFERRED,
    }
}

/// Escalado actual, según Windows.
pub fn scaling(gdi_name: &str) -> WinResult<Scaling> {
    let (paths, _, index) = find_path(gdi_name)?;
    Ok(from_ccd(paths[index].targetInfo.scaling))
}

/// Pide otro escalado para la configuración actual (temporal: sin
/// `SDC_SAVE_TO_DATABASE`). El driver puede ignorarlo: hay que releer.
pub fn set_scaling(gdi_name: &str, scaling: Scaling) -> WinResult<()> {
    let (mut paths, modes, index) = find_path(gdi_name)?;
    paths[index].targetInfo.scaling = to_ccd(scaling);
    // SAFETY: las rutas y los modos vienen de QueryDisplayConfig y siguen
    // vivos durante la llamada.
    let result = unsafe {
        SetDisplayConfig(
            Some(&paths),
            Some(&modes),
            SDC_APPLY | SDC_USE_SUPPLIED_DISPLAY_CONFIG | SDC_ALLOW_CHANGES,
        )
    };
    if result == ERROR_SUCCESS.0 as i32 {
        Ok(())
    } else {
        Err(WinError::DisplayChange {
            api: "SetDisplayConfig",
            code: result,
        })
    }
}

/// Aplica el modo temporal y, si el escalado no quedó como se pidió, lo pide
/// por CCD. Devuelve el escalado que informa Windows al final.
pub fn apply_scaled(gdi_name: &str, request: &ModeRequest) -> WinResult<Scaling> {
    apply_temporary(gdi_name, request)?;
    let wanted = request.scaling;
    let mut current = scaling(gdi_name).unwrap_or_default();
    if matches!(
        wanted,
        Scaling::Stretched | Scaling::AspectRatio | Scaling::Centered
    ) && current != wanted
    {
        let _ = set_scaling(gdi_name, wanted);
        current = scaling(gdi_name).unwrap_or_default();
    }
    Ok(current)
}

/// `true` si se pidió un escalado concreto y Windows informa otro.
pub fn scaling_ignored(wanted: Scaling, got: Scaling) -> bool {
    matches!(
        wanted,
        Scaling::Stretched | Scaling::AspectRatio | Scaling::Centered
    ) && got != wanted
}

/// Resolución nativa del monitor (el modo preferido de su EDID).
pub fn native_mode(gdi_name: &str) -> Option<DisplayMode> {
    let (paths, _, index) = find_path(gdi_name).ok()?;
    let target = &paths[index].targetInfo;
    let mut info = DISPLAYCONFIG_TARGET_PREFERRED_MODE::default();
    if !device_info(
        &mut info,
        DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_PREFERRED_MODE,
        target.adapterId,
        target.id,
    ) || info.width == 0
        || info.height == 0
    {
        return None;
    }
    let sync = info.targetMode.targetVideoSignalInfo.vSyncFreq;
    let refresh_hz = if sync.Denominator == 0 {
        0
    } else {
        (f64::from(sync.Numerator) / f64::from(sync.Denominator)).round() as u32
    };
    Some(DisplayMode::new(info.width, info.height, refresh_hz))
}

fn set_device_info<T>(
    packet: &mut T,
    kind: DISPLAYCONFIG_DEVICE_INFO_TYPE,
    adapter: LUID,
    id: u32,
) -> bool {
    let header = packet as *mut T as *mut DISPLAYCONFIG_DEVICE_INFO_HEADER;
    // SAFETY: los tipos que usamos empiezan con DISPLAYCONFIG_DEVICE_INFO_HEADER
    // (#[repr(C)]) y `size` informa el tamaño real del paquete.
    unsafe {
        (*header).r#type = kind;
        (*header).size = size_of::<T>() as u32;
        (*header).adapterId = adapter;
        (*header).id = id;
        DisplayConfigSetDeviceInfo(header) == ERROR_SUCCESS.0 as i32
    }
}

/// Prende o apaga HDR. Es un ajuste persistente de Windows: VRes guarda el
/// estado anterior y lo restaura al salir.
pub fn set_hdr(gdi_name: &str, enabled: bool) -> WinResult<()> {
    let (paths, _, index) = find_path(gdi_name)?;
    let target = &paths[index].targetInfo;

    let mut packet = SetHdrState {
        header: DISPLAYCONFIG_DEVICE_INFO_HEADER::default(),
        value: u32::from(enabled),
    };
    if set_device_info(
        &mut packet,
        DISPLAYCONFIG_DEVICE_INFO_TYPE(SET_HDR_STATE),
        target.adapterId,
        target.id,
    ) {
        return Ok(());
    }

    // Windows anterior a 24H2.
    let mut legacy = DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE {
        Anonymous: DISPLAYCONFIG_SET_ADVANCED_COLOR_STATE_0 {
            value: u32::from(enabled),
        },
        ..Default::default()
    };
    if set_device_info(
        &mut legacy,
        DISPLAYCONFIG_DEVICE_INFO_SET_ADVANCED_COLOR_STATE,
        target.adapterId,
        target.id,
    ) {
        Ok(())
    } else {
        Err(WinError::Unsupported("hdr state change"))
    }
}
