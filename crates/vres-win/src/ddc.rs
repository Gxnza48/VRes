//! DDC/CI: brillo, contraste y ganancia RGB por hardware, en los monitores
//! externos que lo admiten (Monitor Configuration API, `dxva2.dll`). Cada
//! llamada habla con el monitor por I²C y puede tardar decenas de
//! milisegundos: se usa fuera del hilo del motor y del de la interfaz, y de a
//! una por vez.

use std::collections::BTreeMap;
use std::time::Duration;

use vres_core::monitor::{DdcControl, VcpValue};
use windows::Win32::Devices::Display::{
    DestroyPhysicalMonitors, GetNumberOfPhysicalMonitorsFromHMONITOR,
    GetPhysicalMonitorsFromHMONITOR, GetVCPFeatureAndVCPFeatureReply, PHYSICAL_MONITOR,
    SetVCPFeature,
};
use windows::Win32::Foundation::{HANDLE, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::core::BOOL;

use crate::{WinError, WinResult};

/// Pausa entre una escritura y su relectura (el estándar pide 50 ms).
const SETTLE: Duration = Duration::from_millis(60);

struct Search {
    wanted: Vec<u16>,
    found: Option<HMONITOR>,
}

unsafe extern "system" fn find_monitor(
    monitor: HMONITOR,
    _dc: HDC,
    _rect: *mut RECT,
    data: LPARAM,
) -> BOOL {
    // SAFETY: `data` apunta a la búsqueda, que vive durante EnumDisplayMonitors.
    let search = unsafe { &mut *(data.0 as *mut Search) };
    let mut info = MONITORINFOEXW {
        monitorInfo: MONITORINFO {
            cbSize: size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    // SAFETY: `info` es un MONITORINFOEXW con su tamaño en `cbSize`.
    let ok = unsafe { GetMonitorInfoW(monitor, (&raw mut info).cast::<MONITORINFO>()) };
    if ok.as_bool() {
        let len = info
            .szDevice
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(info.szDevice.len());
        if info.szDevice[..len] == search.wanted[..] {
            search.found = Some(monitor);
            return BOOL(0);
        }
    }
    BOOL(1)
}

/// El monitor de Windows (`HMONITOR`) de una pantalla (`\\.\DISPLAYn`).
fn monitor_handle(gdi_name: &str) -> Option<HMONITOR> {
    let mut search = Search {
        wanted: gdi_name.encode_utf16().collect(),
        found: None,
    };
    // SAFETY: el callback solo usa `search`, que vive durante la llamada.
    let _ = unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(find_monitor),
            LPARAM((&raw mut search) as isize),
        )
    };
    search.found
}

/// Escala de Windows de la pantalla (100, 125, 150…).
pub fn scale_percent(gdi_name: &str) -> Option<u32> {
    let monitor = monitor_handle(gdi_name)?;
    let (mut x, mut y) = (0u32, 0u32);
    // SAFETY: las variables viven durante la llamada.
    unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut x, &mut y) }.ok()?;
    Some((x * 100 + 48) / 96)
}

/// Los monitores físicos de una pantalla; se liberan al soltarlos.
struct Physical(Vec<PHYSICAL_MONITOR>);

impl Physical {
    fn open(gdi_name: &str) -> WinResult<Self> {
        let monitor = monitor_handle(gdi_name).ok_or(WinError::Unsupported("display not found"))?;
        let mut count = 0u32;
        // SAFETY: `count` vive durante la llamada.
        unsafe { GetNumberOfPhysicalMonitorsFromHMONITOR(monitor, &mut count) }
            .map_err(|e| WinError::api("GetNumberOfPhysicalMonitorsFromHMONITOR", e))?;
        let mut list = vec![PHYSICAL_MONITOR::default(); count as usize];
        if !list.is_empty() {
            // SAFETY: `list` tiene lugar para `count` monitores.
            unsafe { GetPhysicalMonitorsFromHMONITOR(monitor, &mut list) }
                .map_err(|e| WinError::api("GetPhysicalMonitorsFromHMONITOR", e))?;
        }
        Ok(Self(list))
    }

    fn first(&self) -> Option<HANDLE> {
        self.0.first().map(|monitor| monitor.hPhysicalMonitor)
    }
}

impl Drop for Physical {
    fn drop(&mut self) {
        if !self.0.is_empty() {
            // SAFETY: son los monitores que devolvió GetPhysicalMonitorsFromHMONITOR.
            let _ = unsafe { DestroyPhysicalMonitors(&self.0) };
        }
    }
}

fn read_vcp(handle: HANDLE, code: u8) -> Option<VcpValue> {
    let (mut current, mut max) = (0u32, 0u32);
    // SAFETY: las variables viven durante la llamada; `handle` es un monitor físico abierto.
    let ok = unsafe {
        GetVCPFeatureAndVCPFeatureReply(handle, code, None, &mut current, Some(&mut max))
    };
    (ok != 0 && max > 0).then_some(VcpValue { current, max })
}

/// Lee los controles que responde el monitor. Vacío si no habla DDC/CI (si
/// el brillo no responde, no se insiste con los demás: cada intento fallido
/// puede tardar).
pub fn read_all(gdi_name: &str) -> WinResult<BTreeMap<DdcControl, VcpValue>> {
    let monitors = Physical::open(gdi_name)?;
    let mut out = BTreeMap::new();
    let Some(handle) = monitors.first() else {
        return Ok(out);
    };
    for control in DdcControl::ALL {
        match read_vcp(handle, control.vcp_code()) {
            Some(value) => {
                out.insert(control, value);
            }
            None if control == DdcControl::Brightness => break,
            None => {}
        }
    }
    Ok(out)
}

/// Lee un control. `None` si el monitor no lo responde.
pub fn get(gdi_name: &str, control: DdcControl) -> WinResult<Option<VcpValue>> {
    let monitors = Physical::open(gdi_name)?;
    Ok(monitors
        .first()
        .and_then(|handle| read_vcp(handle, control.vcp_code())))
}

/// Escribe un control y lo relee.
pub fn set(gdi_name: &str, control: DdcControl, value: u32) -> WinResult<VcpValue> {
    let monitors = Physical::open(gdi_name)?;
    let handle = monitors
        .first()
        .ok_or(WinError::Unsupported("no physical monitor"))?;
    // SAFETY: `handle` es un monitor físico abierto.
    if unsafe { SetVCPFeature(handle, control.vcp_code(), value) } == 0 {
        return Err(WinError::last("SetVCPFeature"));
    }
    std::thread::sleep(SETTLE);
    read_vcp(handle, control.vcp_code()).ok_or(WinError::Unsupported("no reply after write"))
}
