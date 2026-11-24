//! Vibrance digital por driver: NVIDIA (NVAPI) y AMD (ADL).
//!
//! Las DLL del driver se cargan **solo desde System32** (contra *DLL
//! planting*). Si no están, no hay backend: no es un error. El backend de cada
//! pantalla sale de la placa que la maneja (LUID → fabricante), así en una
//! notebook híbrida cada monitor usa el suyo.
//!
//! Las estructuras de los drivers se declaran acá en Rust (no se incluyen
//! cabeceras de los SDK). Funciones e IDs: documentación y código público de
//! NVIDIA (NVAPI, MIT) y AMD (ADL); ver `THIRD-PARTY-NOTICES.md`.

use std::collections::HashMap;
use std::ffi::{CString, c_void};
use std::ptr::null_mut;

use vres_core::model::display::{DisplayInfo, GpuVendor};
use vres_core::model::vibrance::{VibranceBackend, VibranceInfo};
use windows::Win32::Foundation::{FreeLibrary, HMODULE};
use windows::Win32::System::LibraryLoader::{
    GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW,
};
use windows::core::{PCSTR, w};

use crate::{WinError, WinResult};

fn load_system_dll(name: windows::core::PCWSTR) -> Option<HMODULE> {
    // SAFETY: solo se busca en System32; un error significa "no hay driver".
    unsafe { LoadLibraryExW(name, None, LOAD_LIBRARY_SEARCH_SYSTEM32) }.ok()
}

fn symbol(module: HMODULE, name: &[u8]) -> Option<*const c_void> {
    // SAFETY: `name` termina en cero; GetProcAddress no retiene el puntero.
    unsafe { GetProcAddress(module, PCSTR(name.as_ptr())) }.map(|f| f as *const c_void)
}

/// Puntero a función de un driver, ya tipado.
///
/// # Safety
/// `F` tiene que ser un puntero a función con la firma real de `pointer`.
unsafe fn typed<F: Copy>(pointer: *const c_void) -> F {
    const { assert!(size_of::<F>() == size_of::<*const c_void>()) };
    // SAFETY: lo garantiza quien llama; el tamaño se verifica al compilar.
    unsafe { std::mem::transmute_copy::<*const c_void, F>(&pointer) }
}

/// Como [`typed`], para funciones que pueden faltar.
///
/// # Safety
/// La misma que [`typed`].
unsafe fn typed_optional<F: Copy>(pointer: Option<*const c_void>) -> Option<F> {
    // SAFETY: lo garantiza quien llama.
    pointer.map(|pointer| unsafe { typed(pointer) })
}

fn driver_error(api: &'static str, code: i32) -> WinError {
    WinError::DisplayChange { api, code }
}

// ---------------------------------------------------------------- NVIDIA ---

const NVAPI_OK: i32 = 0;
const NV_INITIALIZE: u32 = 0x0150_E828;
const NV_UNLOAD: u32 = 0xD22B_DD7E;
const NV_GET_ASSOCIATED_DISPLAY_HANDLE: u32 = 0x35C2_9134;
const NV_GET_DVC_INFO_EX: u32 = 0x0E45_002D;
const NV_SET_DVC_LEVEL_EX: u32 = 0x4A82_C2B1;
const NV_GET_DVC_INFO: u32 = 0x4085_DE45;
const NV_SET_DVC_LEVEL: u32 = 0x1724_09B4;

/// `NV_DISPLAY_DVC_INFO` (versión 1, 16 bytes).
#[repr(C)]
#[derive(Default)]
struct DvcInfo {
    version: u32,
    current: i32,
    min: i32,
    max: i32,
}

/// `NV_DISPLAY_DVC_INFO_EX` (versión 1, 20 bytes).
#[repr(C)]
#[derive(Default)]
struct DvcInfoEx {
    version: u32,
    current: i32,
    min: i32,
    max: i32,
    default: i32,
}

const _: () = assert!(size_of::<DvcInfo>() == 16);
const _: () = assert!(size_of::<DvcInfoEx>() == 20);
const DVC_INFO_VERSION: u32 = size_of::<DvcInfo>() as u32 | (1 << 16);
const DVC_INFO_EX_VERSION: u32 = size_of::<DvcInfoEx>() as u32 | (1 << 16);

type NvQueryInterface = unsafe extern "C" fn(u32) -> *mut c_void;
type NvNoArgs = unsafe extern "C" fn() -> i32;
type NvGetHandle = unsafe extern "C" fn(*const u8, *mut *mut c_void) -> i32;
type NvGetDvcEx = unsafe extern "C" fn(*mut c_void, u32, *mut DvcInfoEx) -> i32;
type NvGetDvc = unsafe extern "C" fn(*mut c_void, u32, *mut DvcInfo) -> i32;
type NvSetDvc = unsafe extern "C" fn(*mut c_void, u32, i32) -> i32;

struct Nvapi {
    module: HMODULE,
    unload: Option<NvNoArgs>,
    get_handle: NvGetHandle,
    get_ex: Option<NvGetDvcEx>,
    set_ex: Option<NvGetDvcEx>,
    get: Option<NvGetDvc>,
    set: Option<NvSetDvc>,
}

impl Nvapi {
    fn load() -> Option<Self> {
        let module = load_system_dll(w!("nvapi64.dll"))?;
        let loaded = Self::bind(module);
        if loaded.is_none() {
            // SAFETY: el módulo es nuestro y nadie más lo usa.
            let _ = unsafe { FreeLibrary(module) };
        }
        loaded
    }

    fn bind(module: HMODULE) -> Option<Self> {
        let query = symbol(module, b"nvapi_QueryInterface\0")?;
        // SAFETY: `nvapi_QueryInterface` tiene esta firma en todas las
        // versiones públicas de NVAPI.
        let query: NvQueryInterface = unsafe { typed(query) };
        let function = |id: u32| -> Option<*const c_void> {
            // SAFETY: devuelve un puntero a función o null si no existe.
            let pointer = unsafe { query(id) };
            (!pointer.is_null()).then_some(pointer.cast_const())
        };

        // SAFETY: (vale para todas las conversiones de acá) cada ID corresponde
        // a la firma declarada en su tipo, según la documentación pública de NVAPI.
        let initialize: NvNoArgs = unsafe { typed(function(NV_INITIALIZE)?) };
        // SAFETY: inicializa la biblioteca (con conteo de referencias).
        if unsafe { initialize() } != NVAPI_OK {
            return None;
        }
        // SAFETY: ver arriba.
        let get_handle: NvGetHandle = unsafe { typed(function(NV_GET_ASSOCIATED_DISPLAY_HANDLE)?) };
        // SAFETY: ver arriba.
        let unload: Option<NvNoArgs> = unsafe { typed_optional(function(NV_UNLOAD)) };
        // SAFETY: ver arriba.
        let get_ex: Option<NvGetDvcEx> = unsafe { typed_optional(function(NV_GET_DVC_INFO_EX)) };
        // SAFETY: ver arriba (la escritura usa la misma firma que la lectura).
        let set_ex: Option<NvGetDvcEx> = unsafe { typed_optional(function(NV_SET_DVC_LEVEL_EX)) };
        // SAFETY: ver arriba.
        let get: Option<NvGetDvc> = unsafe { typed_optional(function(NV_GET_DVC_INFO)) };
        // SAFETY: ver arriba.
        let set: Option<NvSetDvc> = unsafe { typed_optional(function(NV_SET_DVC_LEVEL)) };
        Some(Self {
            module,
            unload,
            get_handle,
            get_ex,
            set_ex,
            get,
            set,
        })
    }

    fn handle(&self, gdi_name: &str) -> Option<*mut c_void> {
        let name = CString::new(gdi_name).ok()?;
        let mut handle = null_mut();
        // SAFETY: `name` termina en cero y `handle` vive durante la llamada.
        let status = unsafe { (self.get_handle)(name.as_ptr().cast(), &mut handle) };
        (status == NVAPI_OK && !handle.is_null()).then_some(handle)
    }

    fn read_ex(&self, handle: *mut c_void) -> Option<DvcInfoEx> {
        let get_ex = self.get_ex?;
        let mut info = DvcInfoEx {
            version: DVC_INFO_EX_VERSION,
            ..DvcInfoEx::default()
        };
        // SAFETY: estructura con su versión y tamaño correctos.
        let status = unsafe { get_ex(handle, 0, &mut info) };
        // min = max = 0 significa "no soportado".
        (status == NVAPI_OK && info.max > info.min).then_some(info)
    }

    fn info(&self, gdi_name: &str) -> Option<VibranceInfo> {
        let handle = self.handle(gdi_name)?;
        if let Some(info) = self.read_ex(handle) {
            return Some(VibranceInfo {
                backend: VibranceBackend::Nvidia,
                current: info.current,
                min: info.min,
                max: info.max,
                default: info.default,
            });
        }
        // Variante clásica: 0 es el normal y no se puede bajar.
        let get = self.get?;
        let mut info = DvcInfo {
            version: DVC_INFO_VERSION,
            ..DvcInfo::default()
        };
        // SAFETY: estructura con su versión y tamaño correctos.
        let status = unsafe { get(handle, 0, &mut info) };
        (status == NVAPI_OK && info.max > info.min).then_some(VibranceInfo {
            backend: VibranceBackend::Nvidia,
            current: info.current,
            min: info.min,
            max: info.max,
            default: info.min,
        })
    }

    fn set(&self, gdi_name: &str, raw: i32) -> WinResult<()> {
        let handle = self
            .handle(gdi_name)
            .ok_or(WinError::Unsupported("nvidia display handle"))?;
        if let (Some(set_ex), Some(mut info)) = (self.set_ex, self.read_ex(handle)) {
            info.current = raw.clamp(info.min, info.max);
            // SAFETY: estructura leída recién, con su versión.
            let status = unsafe { set_ex(handle, 0, &mut info) };
            return if status == NVAPI_OK {
                Ok(())
            } else {
                Err(driver_error("NvAPI_SetDVCLevelEx", status))
            };
        }
        let set = self.set.ok_or(WinError::Unsupported("NvAPI_SetDVCLevel"))?;
        // SAFETY: handle válido; el nivel se recorta en el driver.
        let status = unsafe { set(handle, 0, raw) };
        if status == NVAPI_OK {
            Ok(())
        } else {
            Err(driver_error("NvAPI_SetDVCLevel", status))
        }
    }
}

impl Drop for Nvapi {
    fn drop(&mut self) {
        if let Some(unload) = self.unload {
            // SAFETY: cierra la inicialización que hicimos en `bind`.
            unsafe { unload() };
        }
        // SAFETY: el módulo es nuestro; no queda ningún puntero a sus funciones.
        let _ = unsafe { FreeLibrary(self.module) };
    }
}

// ------------------------------------------------------------------- AMD ---

const ADL_OK: i32 = 0;
const ADL_DISPLAY_COLOR_SATURATION: i32 = 4;
const ADL_DISPLAY_CONNECTED: i32 = 0x1;
const ADL_DISPLAY_MAPPED: i32 = 0x2;

/// `AdapterInfo` de ADL (1572 bytes).
#[repr(C)]
struct AdapterInfo {
    size: i32,
    adapter_index: i32,
    udid: [u8; 256],
    bus_number: i32,
    device_number: i32,
    function_number: i32,
    vendor_id: i32,
    adapter_name: [u8; 256],
    display_name: [u8; 256],
    present: i32,
    exist: i32,
    driver_path: [u8; 256],
    driver_path_ext: [u8; 256],
    pnp_string: [u8; 256],
    os_display_index: i32,
}

/// `ADLDisplayID`.
#[repr(C)]
struct AdlDisplayId {
    logical_index: i32,
    physical_index: i32,
    logical_adapter_index: i32,
    physical_adapter_index: i32,
}

/// `ADLDisplayInfo` (552 bytes).
#[repr(C)]
struct AdlDisplayInfo {
    display_id: AdlDisplayId,
    controller_index: i32,
    display_name: [u8; 256],
    manufacturer_name: [u8; 256],
    display_type: i32,
    output_type: i32,
    connector: i32,
    info_mask: i32,
    info_value: i32,
}

const _: () = assert!(size_of::<AdapterInfo>() == 1572);
const _: () = assert!(size_of::<AdlDisplayInfo>() == 552);

type AdlMalloc = unsafe extern "C" fn(i32) -> *mut c_void;
type AdlCreate = unsafe extern "C" fn(AdlMalloc, i32, *mut *mut c_void) -> i32;
type AdlDestroy = unsafe extern "C" fn(*mut c_void) -> i32;
type AdlCount = unsafe extern "C" fn(*mut c_void, *mut i32) -> i32;
type AdlAdapters = unsafe extern "C" fn(*mut c_void, *mut AdapterInfo, i32) -> i32;
type AdlDisplays =
    unsafe extern "C" fn(*mut c_void, i32, *mut i32, *mut *mut AdlDisplayInfo, i32) -> i32;
type AdlColorGet = unsafe extern "C" fn(
    *mut c_void,
    i32,
    i32,
    i32,
    *mut i32,
    *mut i32,
    *mut i32,
    *mut i32,
    *mut i32,
) -> i32;
type AdlColorSet = unsafe extern "C" fn(*mut c_void, i32, i32, i32, i32) -> i32;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
}

/// ADL reserva la memoria que devuelve con esta función; VRes la libera con `free`.
unsafe extern "C" fn adl_alloc(size: i32) -> *mut c_void {
    // SAFETY: tamaño pedido por ADL; si falla, malloc devuelve null.
    unsafe { malloc(size.max(0) as usize) }
}

fn c_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

struct Adl {
    module: HMODULE,
    context: *mut c_void,
    destroy: AdlDestroy,
    count: AdlCount,
    adapters: AdlAdapters,
    displays: AdlDisplays,
    color_get: AdlColorGet,
    color_set: AdlColorSet,
    /// Nombre GDI → (adaptador, pantalla) de ADL.
    located: HashMap<String, (i32, i32)>,
}

impl Adl {
    fn load() -> Option<Self> {
        let module = load_system_dll(w!("atiadlxx.dll"))?;
        let loaded = Self::bind(module);
        if loaded.is_none() {
            // SAFETY: el módulo es nuestro y nadie más lo usa.
            let _ = unsafe { FreeLibrary(module) };
        }
        loaded
    }

    fn bind(module: HMODULE) -> Option<Self> {
        let get = |name: &[u8]| symbol(module, name);
        // Primero todas las funciones; recién después se crea el contexto
        // (así nunca queda un contexto sin destruir).
        // SAFETY: (vale para todas las conversiones de acá) cada función
        // exportada tiene la firma declarada en su tipo, según el SDK de ADL.
        let create: AdlCreate = unsafe { typed(get(b"ADL2_Main_Control_Create\0")?) };
        // SAFETY: ver arriba.
        let destroy: AdlDestroy = unsafe { typed(get(b"ADL2_Main_Control_Destroy\0")?) };
        // SAFETY: ver arriba.
        let count: AdlCount = unsafe { typed(get(b"ADL2_Adapter_NumberOfAdapters_Get\0")?) };
        // SAFETY: ver arriba.
        let adapters: AdlAdapters = unsafe { typed(get(b"ADL2_Adapter_AdapterInfo_Get\0")?) };
        // SAFETY: ver arriba.
        let displays: AdlDisplays = unsafe { typed(get(b"ADL2_Display_DisplayInfo_Get\0")?) };
        // SAFETY: ver arriba.
        let color_get: AdlColorGet = unsafe { typed(get(b"ADL2_Display_Color_Get\0")?) };
        // SAFETY: ver arriba.
        let color_set: AdlColorSet = unsafe { typed(get(b"ADL2_Display_Color_Set\0")?) };

        let mut context = null_mut();
        // SAFETY: `adl_alloc` vive toda la ejecución; 1 = solo adaptadores conectados.
        if unsafe { create(adl_alloc, 1, &mut context) } != ADL_OK || context.is_null() {
            return None;
        }
        Some(Self {
            module,
            context,
            destroy,
            count,
            adapters,
            displays,
            color_get,
            color_set,
            located: HashMap::new(),
        })
    }

    /// (adaptador, pantalla) de ADL para un nombre GDI.
    fn locate(&mut self, gdi_name: &str) -> Option<(i32, i32)> {
        if let Some(found) = self.located.get(gdi_name) {
            return Some(*found);
        }
        let mut count = 0;
        // SAFETY: contexto válido; `count` vive durante la llamada.
        if unsafe { (self.count)(self.context, &mut count) } != ADL_OK || count <= 0 {
            return None;
        }
        // SAFETY: AdapterInfo es una estructura de enteros y bytes: todo cero es válido.
        let mut adapters: Vec<AdapterInfo> =
            (0..count).map(|_| unsafe { std::mem::zeroed() }).collect();
        let bytes = (adapters.len() * size_of::<AdapterInfo>()) as i32;
        // SAFETY: el vector tiene exactamente `bytes` bytes.
        if unsafe { (self.adapters)(self.context, adapters.as_mut_ptr(), bytes) } != ADL_OK {
            return None;
        }

        let mut found = None;
        for adapter in adapters.iter().filter(|adapter| {
            adapter.present != 0 && c_string(&adapter.display_name).eq_ignore_ascii_case(gdi_name)
        }) {
            let mut listed = 0;
            let mut list: *mut AdlDisplayInfo = null_mut();
            // SAFETY: ADL reserva `list` con `adl_alloc`; lo liberamos abajo.
            let status = unsafe {
                (self.displays)(
                    self.context,
                    adapter.adapter_index,
                    &mut listed,
                    &mut list,
                    0,
                )
            };
            if status != ADL_OK || list.is_null() {
                continue;
            }
            // SAFETY: ADL devolvió `listed` estructuras contiguas en `list`.
            let entries = unsafe { std::slice::from_raw_parts(list, listed.max(0) as usize) };
            let ready = |entry: &&AdlDisplayInfo| {
                entry.display_id.logical_index >= 0
                    && entry.info_value & (ADL_DISPLAY_CONNECTED | ADL_DISPLAY_MAPPED)
                        == ADL_DISPLAY_CONNECTED | ADL_DISPLAY_MAPPED
            };
            let pick = entries
                .iter()
                .filter(ready)
                .find(|entry| entry.display_id.logical_adapter_index == adapter.adapter_index)
                .or_else(|| entries.iter().find(ready))
                .map(|entry| {
                    (
                        entry.display_id.logical_adapter_index,
                        entry.display_id.logical_index,
                    )
                });
            // SAFETY: memoria reservada por `adl_alloc` (malloc).
            unsafe { free(list.cast()) };
            if pick.is_some() {
                found = pick;
                break;
            }
        }
        if let Some(found) = found {
            self.located.insert(gdi_name.to_owned(), found);
        }
        found
    }

    fn info(&mut self, gdi_name: &str) -> Option<VibranceInfo> {
        let (adapter, display) = self.locate(gdi_name)?;
        let (mut current, mut default, mut min, mut max, mut step) = (0, 0, 0, 0, 0);
        // SAFETY: contexto válido; todas las salidas viven durante la llamada.
        let status = unsafe {
            (self.color_get)(
                self.context,
                adapter,
                display,
                ADL_DISPLAY_COLOR_SATURATION,
                &mut current,
                &mut default,
                &mut min,
                &mut max,
                &mut step,
            )
        };
        (status == ADL_OK && max > min).then_some(VibranceInfo {
            backend: VibranceBackend::Amd,
            current,
            min,
            max,
            default,
        })
    }

    fn set(&mut self, gdi_name: &str, raw: i32) -> WinResult<()> {
        let (adapter, display) = self
            .locate(gdi_name)
            .ok_or(WinError::Unsupported("amd display index"))?;
        // SAFETY: contexto válido; el driver valida el rango.
        let status = unsafe {
            (self.color_set)(
                self.context,
                adapter,
                display,
                ADL_DISPLAY_COLOR_SATURATION,
                raw,
            )
        };
        if status == ADL_OK {
            Ok(())
        } else {
            Err(driver_error("ADL2_Display_Color_Set", status))
        }
    }
}

impl Drop for Adl {
    fn drop(&mut self) {
        // SAFETY: contexto creado en `bind`; se destruye una sola vez.
        unsafe { (self.destroy)(self.context) };
        // SAFETY: el módulo es nuestro; no queda ningún puntero a sus funciones.
        let _ = unsafe { FreeLibrary(self.module) };
    }
}

// --------------------------------------------------------------- público ---

/// Vibrance por driver de todas las pantallas. Vive en un solo hilo (no es
/// `Send`): lo crea y lo usa el hilo del motor.
pub struct DriverVibrance {
    nvidia: Option<Nvapi>,
    amd: Option<Adl>,
}

impl std::fmt::Debug for DriverVibrance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DriverVibrance")
            .field("nvidia", &self.nvidia.is_some())
            .field("amd", &self.amd.is_some())
            .finish()
    }
}

impl DriverVibrance {
    /// Carga los drivers que haya instalados.
    pub fn load() -> Self {
        Self {
            nvidia: Nvapi::load(),
            amd: Adl::load(),
        }
    }

    /// Qué drivers se cargaron (para el diagnóstico).
    pub fn loaded(&self) -> (bool, bool) {
        (self.nvidia.is_some(), self.amd.is_some())
    }

    /// Olvida lo que sabía de las pantallas (después de un cambio de pantallas).
    pub fn forget_displays(&mut self) {
        if let Some(amd) = &mut self.amd {
            amd.located.clear();
        }
    }

    /// Rango y valor actual de la pantalla, o `None` si su driver no tiene vibrance.
    pub fn info(&mut self, display: &DisplayInfo) -> Option<VibranceInfo> {
        match display.adapter.vendor {
            GpuVendor::Nvidia => self.nvidia.as_ref()?.info(&display.gdi_name),
            GpuVendor::Amd => self.amd.as_mut()?.info(&display.gdi_name),
            GpuVendor::Intel | GpuVendor::Other => None,
        }
    }

    /// Escribe un nivel del driver (no lo verifica: hay que releer con `info`).
    pub fn set(&mut self, display: &DisplayInfo, raw: i32) -> WinResult<()> {
        match display.adapter.vendor {
            GpuVendor::Nvidia => self
                .nvidia
                .as_ref()
                .ok_or(WinError::Unsupported("nvapi"))?
                .set(&display.gdi_name, raw),
            GpuVendor::Amd => self
                .amd
                .as_mut()
                .ok_or(WinError::Unsupported("adl"))?
                .set(&display.gdi_name, raw),
            GpuVendor::Intel | GpuVendor::Other => Err(WinError::Unsupported("vibrance backend")),
        }
    }
}
