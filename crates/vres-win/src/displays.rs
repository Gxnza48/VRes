//! Enumeración de pantallas con la API de configuración de pantalla (CCD).
//!
//! Por cada pantalla activa: nombre GDI (`\\.\DISPLAYn`), nombre real del
//! monitor, ruta de dispositivo (base del identificador estable), frecuencia
//! exacta, posición, GPU que la maneja (cruzando el LUID del adaptador con
//! DXGI) y estado de HDR.

use std::collections::HashMap;

use vres_core::model::display::{
    AdapterInfo, DisplayInfo, GpuVendor, HdrState, RectPx, stable_display_id,
};
use windows::Win32::Devices::Display::{
    DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO, DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
    DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME, DISPLAYCONFIG_DEVICE_INFO_HEADER,
    DISPLAYCONFIG_DEVICE_INFO_TYPE, DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO, DISPLAYCONFIG_MODE_INFO,
    DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SOURCE_DEVICE_NAME,
    DISPLAYCONFIG_TARGET_DEVICE_NAME, DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes,
    QDC_ONLY_ACTIVE_PATHS, QueryDisplayConfig,
};
use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, LUID};
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1};

use crate::{WinError, WinResult};

/// Tecnologías de salida que corresponden al panel interno de una notebook.
const INTERNAL_OUTPUTS: [u32; 4] = [
    0x8000_0000, // INTERNAL
    11,          // DISPLAYPORT_EMBEDDED
    13,          // UDI_EMBEDDED
    6,           // LVDS
];

/// `DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO_2` (Windows 11 24H2+).
/// No está en el crate `windows`: se declara a mano.
const GET_ADVANCED_COLOR_INFO_2: i32 = 15;

#[repr(C)]
#[derive(Default)]
struct AdvancedColorInfo2 {
    header: DISPLAYCONFIG_DEVICE_INFO_HEADER,
    /// Bits: 0 advancedColorSupported, 1 advancedColorActive,
    /// 3 advancedColorLimitedByPolicy, 4 highDynamicRangeSupported,
    /// 5 highDynamicRangeUserEnabled, 6 wideColorSupported, 7 wideColorUserEnabled.
    flags: u32,
    color_encoding: i32,
    bits_per_color_channel: u32,
    /// 0 SDR, 1 WCG, 2 HDR.
    active_color_mode: i32,
}

fn utf16_to_string(buffer: &[u16]) -> String {
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

fn luid_key(luid: LUID) -> (u32, i32) {
    (luid.LowPart, luid.HighPart)
}

/// Pide información de un dispositivo; `T` empieza con el encabezado.
fn device_info<T>(
    packet: &mut T,
    kind: DISPLAYCONFIG_DEVICE_INFO_TYPE,
    adapter: LUID,
    id: u32,
) -> bool {
    let header = packet as *mut T as *mut DISPLAYCONFIG_DEVICE_INFO_HEADER;
    // SAFETY: todos los tipos que usamos empiezan con
    // DISPLAYCONFIG_DEVICE_INFO_HEADER (#[repr(C)]), y `size` informa el
    // tamaño real del paquete, que vive durante toda la llamada.
    unsafe {
        (*header).r#type = kind;
        (*header).size = size_of::<T>() as u32;
        (*header).adapterId = adapter;
        (*header).id = id;
        DisplayConfigGetDeviceInfo(header) == ERROR_SUCCESS.0 as i32
    }
}

/// Placas de video del sistema, por LUID.
fn adapters_by_luid() -> HashMap<(u32, i32), AdapterInfo> {
    let mut adapters = HashMap::new();
    // SAFETY: CreateDXGIFactory1 devuelve un objeto COM con conteo de referencias.
    let Ok(factory) = (unsafe { CreateDXGIFactory1::<IDXGIFactory1>() }) else {
        return adapters;
    };
    for index in 0.. {
        // SAFETY: EnumAdapters1 devuelve error al pasar el último adaptador.
        let Ok(adapter) = (unsafe { factory.EnumAdapters1(index) }) else {
            break;
        };
        // SAFETY: GetDesc1 completa una estructura que devuelve por valor.
        if let Ok(desc) = unsafe { adapter.GetDesc1() } {
            adapters.insert(
                luid_key(desc.AdapterLuid),
                AdapterInfo {
                    vendor: GpuVendor::from_pci_vendor_id(desc.VendorId),
                    name: utf16_to_string(&desc.Description),
                },
            );
        }
    }
    adapters
}

fn query_active_paths() -> WinResult<(Vec<DISPLAYCONFIG_PATH_INFO>, Vec<DISPLAYCONFIG_MODE_INFO>)> {
    // La configuración puede cambiar entre las dos llamadas: se reintenta.
    for _ in 0..5 {
        let (mut path_count, mut mode_count) = (0u32, 0u32);
        // SAFETY: las dos variables viven durante la llamada.
        let sizes = unsafe {
            GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
        };
        if sizes != ERROR_SUCCESS {
            return Err(WinError::api(
                "GetDisplayConfigBufferSizes",
                windows::core::Error::from(sizes.to_hresult()),
            ));
        }
        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];
        // SAFETY: los vectores tienen el tamaño que Windows pidió y los
        // contadores informan su capacidad.
        let result = unsafe {
            QueryDisplayConfig(
                QDC_ONLY_ACTIVE_PATHS,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                None,
            )
        };
        if result == ERROR_INSUFFICIENT_BUFFER {
            continue;
        }
        if result != ERROR_SUCCESS {
            return Err(WinError::api(
                "QueryDisplayConfig",
                windows::core::Error::from(result.to_hresult()),
            ));
        }
        paths.truncate(path_count as usize);
        modes.truncate(mode_count as usize);
        return Ok((paths, modes));
    }
    Err(WinError::Timeout("QueryDisplayConfig"))
}

fn hdr_state(adapter: LUID, target_id: u32) -> HdrState {
    let mut info2 = AdvancedColorInfo2::default();
    if device_info(
        &mut info2,
        DISPLAYCONFIG_DEVICE_INFO_TYPE(GET_ADVANCED_COLOR_INFO_2),
        adapter,
        target_id,
    ) {
        let supported = info2.flags & (1 << 4) != 0;
        let user_enabled = info2.flags & (1 << 5) != 0;
        let active = info2.flags & (1 << 1) != 0;
        return HdrState {
            supported,
            enabled: active && user_enabled && info2.active_color_mode == 2,
        };
    }

    // Windows anterior a 24H2.
    let mut info = DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO::default();
    if device_info(
        &mut info,
        DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
        adapter,
        target_id,
    ) {
        // SAFETY: lectura de la unión de bits que Windows acaba de completar.
        let flags = unsafe { info.Anonymous.value };
        let supported = flags & 1 != 0;
        let enabled = flags & (1 << 1) != 0;
        let wide_color_enforced = flags & (1 << 2) != 0;
        return HdrState {
            supported,
            enabled: enabled && !wide_color_enforced,
        };
    }
    HdrState::default()
}

/// Pantallas activas, ordenadas: la principal primero y después de izquierda
/// a derecha.
pub fn enumerate() -> WinResult<Vec<DisplayInfo>> {
    let (paths, modes) = query_active_paths()?;
    let adapters = adapters_by_luid();
    let mut displays = Vec::with_capacity(paths.len());

    for path in &paths {
        let source = &path.sourceInfo;
        let target = &path.targetInfo;

        let mut source_name = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
        if !device_info(
            &mut source_name,
            DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME,
            source.adapterId,
            source.id,
        ) {
            continue;
        }
        let gdi_name = utf16_to_string(&source_name.viewGdiDeviceName);

        let mut target_name = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
        let has_target = device_info(
            &mut target_name,
            DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
            target.adapterId,
            target.id,
        );
        let friendly_name = if has_target {
            // El EDID suele rellenar el nombre con espacios.
            utf16_to_string(&target_name.monitorFriendlyDeviceName)
                .trim()
                .to_owned()
        } else {
            String::new()
        };
        let device_path = if has_target {
            utf16_to_string(&target_name.monitorDevicePath)
        } else {
            gdi_name.clone()
        };
        let internal =
            has_target && INTERNAL_OUTPUTS.contains(&(target_name.outputTechnology.0 as u32));

        // SAFETY: con QDC_ONLY_ACTIVE_PATHS (sin "virtual mode aware") la unión
        // contiene el índice del modo de origen.
        let mode_index = unsafe { source.Anonymous.modeInfoIdx } as usize;
        let bounds = modes
            .get(mode_index)
            .filter(|mode| mode.infoType == DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE)
            .map(|mode| {
                // SAFETY: infoType confirma que la unión es un modo de origen.
                let source_mode = unsafe { mode.Anonymous.sourceMode };
                RectPx {
                    x: source_mode.position.x,
                    y: source_mode.position.y,
                    width: source_mode.width,
                    height: source_mode.height,
                }
            })
            .unwrap_or(RectPx {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            });

        let refresh = target.refreshRate;
        let refresh_hz = if refresh.Denominator == 0 {
            0.0
        } else {
            f64::from(refresh.Numerator) / f64::from(refresh.Denominator)
        };

        let adapter = adapters
            .get(&luid_key(target.adapterId))
            .cloned()
            .unwrap_or(AdapterInfo {
                vendor: GpuVendor::Other,
                name: String::new(),
            });

        displays.push(DisplayInfo {
            id: stable_display_id(&device_path),
            gdi_name,
            friendly_name,
            adapter,
            primary: bounds.x == 0 && bounds.y == 0,
            bounds,
            internal,
            refresh_hz,
            hdr: hdr_state(target.adapterId, target.id),
        });
    }

    displays.sort_by_key(|display| (!display.primary, display.bounds.x, display.bounds.y));
    Ok(displays)
}
