//! Brillo del panel de una notebook, por WMI (`root\WMI`,
//! `WmiMonitorBrightness`): los paneles internos no hablan DDC/CI. Es el mismo
//! brillo que el control de Windows.

use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
    CoSetProxyBlanket, CoUninitialize, EOAC_NONE, RPC_C_AUTHN_LEVEL_CALL,
    RPC_C_IMP_LEVEL_IMPERSONATE,
};
use windows::Win32::System::Variant::{
    VARIANT, VT_BSTR, VT_I4, VT_UI1, VariantClear, VariantToUInt32,
};
use windows::Win32::System::Wmi::{
    IWbemClassObject, IWbemLocator, IWbemServices, WBEM_FLAG_FORWARD_ONLY,
    WBEM_FLAG_RETURN_IMMEDIATELY, WBEM_GENERIC_FLAG_TYPE, WBEM_INFINITE, WbemLocator,
};
use windows::core::{BSTR, PCWSTR, w};

/// `RPC_C_AUTHN_WINNT` y `RPC_C_AUTHZ_NONE` (rpcdce.h).
const RPC_C_AUTHN_WINNT: u32 = 10;
const RPC_C_AUTHZ_NONE: u32 = 0;

use crate::{WinError, WinResult};

/// COM inicializado en este hilo mientras vive (y liberado si lo inicializó).
struct Com {
    owned: bool,
}

impl Com {
    fn init() -> Self {
        // SAFETY: inicializa COM en este hilo; si ya estaba inicializado en
        // otro modo, se usa como está y no se libera.
        let result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        Self {
            owned: result.is_ok(),
        }
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        if self.owned {
            // SAFETY: equilibra el CoInitializeEx exitoso de `init`.
            unsafe { CoUninitialize() };
        }
    }
}

/// Un VARIANT que se libera al soltarlo.
struct Variant(VARIANT);

impl Variant {
    fn empty() -> Self {
        Self(VARIANT::default())
    }

    fn i32(value: i32) -> Self {
        let mut variant = VARIANT::default();
        // SAFETY: se escribe un entero (sin memoria propia) y su tipo.
        unsafe {
            let inner = &mut *variant.Anonymous.Anonymous;
            inner.vt = VT_I4;
            inner.Anonymous.lVal = value;
        }
        Self(variant)
    }

    fn u8(value: u8) -> Self {
        let mut variant = VARIANT::default();
        // SAFETY: se escribe un byte (sin memoria propia) y su tipo.
        unsafe {
            let inner = &mut *variant.Anonymous.Anonymous;
            inner.vt = VT_UI1;
            inner.Anonymous.bVal = value;
        }
        Self(variant)
    }

    fn to_u32(&self) -> Option<u32> {
        // SAFETY: `self.0` es un VARIANT válido.
        unsafe { VariantToUInt32(&self.0) }.ok()
    }

    fn to_bstr(&self) -> Option<BSTR> {
        // SAFETY: se lee el tipo y, si es una cadena, se copia.
        unsafe {
            let inner = &*self.0.Anonymous.Anonymous;
            (inner.vt == VT_BSTR).then(|| BSTR::clone(&inner.Anonymous.bstrVal))
        }
    }
}

impl Drop for Variant {
    fn drop(&mut self) {
        // SAFETY: libera lo que el VARIANT tenga reservado (nada, para números).
        let _ = unsafe { VariantClear(&mut self.0) };
    }
}

fn services() -> WinResult<IWbemServices> {
    // SAFETY: crea el localizador de WMI (objeto COM del sistema, en proceso).
    let locator: IWbemLocator =
        unsafe { CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER) }
            .map_err(|e| WinError::com("CoCreateInstance(WbemLocator)", &e))?;
    // SAFETY: conexión local, con parámetros válidos.
    let services = unsafe {
        locator.ConnectServer(
            &BSTR::from(r"ROOT\WMI"),
            &BSTR::new(),
            &BSTR::new(),
            &BSTR::new(),
            0,
            &BSTR::new(),
            None,
        )
    }
    .map_err(|e| WinError::com("IWbemLocator::ConnectServer", &e))?;
    // `root\WMI` rechaza el nivel por defecto ("identificar"): se pide
    // suplantar, como hace cualquier cliente de WMI.
    // SAFETY: `services` es el proxy recién creado; parámetros válidos.
    unsafe {
        CoSetProxyBlanket(
            &services,
            RPC_C_AUTHN_WINNT,
            RPC_C_AUTHZ_NONE,
            PCWSTR::null(),
            RPC_C_AUTHN_LEVEL_CALL,
            RPC_C_IMP_LEVEL_IMPERSONATE,
            None,
            EOAC_NONE,
        )
    }
    .map_err(|e| WinError::com("CoSetProxyBlanket", &e))?;
    Ok(services)
}

fn first(services: &IWbemServices, query: &str) -> WinResult<Option<IWbemClassObject>> {
    // SAFETY: consulta WQL de solo lectura.
    let rows = unsafe {
        services.ExecQuery(
            &BSTR::from("WQL"),
            &BSTR::from(query),
            WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY,
            None,
        )
    }
    .map_err(|e| WinError::com("IWbemServices::ExecQuery", &e))?;
    let mut objects = [None];
    let mut returned = 0u32;
    // SAFETY: `objects` y `returned` viven durante la llamada.
    let status = unsafe { rows.Next(WBEM_INFINITE, &mut objects, &mut returned) };
    if status.is_err() {
        return Err(WinError::Com {
            api: "IEnumWbemClassObject::Next",
            code: status.0 as u32,
        });
    }
    Ok(if returned == 1 {
        objects[0].take()
    } else {
        None
    })
}

fn property(object: &IWbemClassObject, name: windows::core::PCWSTR) -> WinResult<Variant> {
    let mut value = Variant::empty();
    // SAFETY: `value` vive durante la llamada y se libera al soltarlo.
    unsafe { object.Get(name, 0, &mut value.0, None, None) }
        .map_err(|e| WinError::com("IWbemClassObject::Get", &e))?;
    Ok(value)
}

/// Brillo actual del panel (0–100). `None` si no hay un panel con brillo
/// por WMI (por ejemplo, en una PC de escritorio).
pub fn brightness() -> WinResult<Option<u32>> {
    let _com = Com::init();
    let services = services()?;
    let Some(object) = first(
        &services,
        "SELECT CurrentBrightness FROM WmiMonitorBrightness WHERE Active = TRUE",
    )?
    else {
        return Ok(None);
    };
    Ok(property(&object, w!("CurrentBrightness"))?.to_u32())
}

/// Cambia el brillo del panel (0–100).
pub fn set_brightness(value: u32) -> WinResult<()> {
    let _com = Com::init();
    let services = services()?;
    let instance = first(
        &services,
        "SELECT * FROM WmiMonitorBrightnessMethods WHERE Active = TRUE",
    )?
    .ok_or(WinError::Unsupported("panel brightness"))?;
    let path = property(&instance, w!("__PATH"))?
        .to_bstr()
        .ok_or(WinError::Unsupported("panel brightness path"))?;

    let mut class = None;
    // SAFETY: `class` vive durante la llamada.
    unsafe {
        services.GetObject(
            &BSTR::from("WmiMonitorBrightnessMethods"),
            WBEM_GENERIC_FLAG_TYPE(0),
            None,
            Some(&mut class),
            None,
        )
    }
    .map_err(|e| WinError::com("IWbemServices::GetObject", &e))?;
    let class = class.ok_or(WinError::Unsupported("panel brightness class"))?;
    let mut input = None;
    // SAFETY: `input` vive durante la llamada; la firma de salida no hace falta.
    unsafe { class.GetMethod(w!("WmiSetBrightness"), 0, &mut input, std::ptr::null_mut()) }
        .map_err(|e| WinError::com("IWbemClassObject::GetMethod", &e))?;
    let input = input.ok_or(WinError::Unsupported("panel brightness method"))?;
    // SAFETY: crea los parámetros de entrada del método.
    let params = unsafe { input.SpawnInstance(0) }
        .map_err(|e| WinError::com("IWbemClassObject::SpawnInstance", &e))?;

    let timeout = Variant::i32(1);
    let brightness = Variant::u8(value.min(100) as u8);
    // SAFETY: los VARIANT viven durante las llamadas.
    unsafe {
        params
            .Put(w!("Timeout"), 0, &timeout.0, 0)
            .map_err(|e| WinError::com("IWbemClassObject::Put(Timeout)", &e))?;
        params
            .Put(w!("Brightness"), 0, &brightness.0, 0)
            .map_err(|e| WinError::com("IWbemClassObject::Put(Brightness)", &e))?;
    }
    // SAFETY: ruta, método y parámetros válidos.
    unsafe {
        services.ExecMethod(
            &path,
            &BSTR::from("WmiSetBrightness"),
            WBEM_GENERIC_FLAG_TYPE(0),
            None,
            &params,
            None,
            None,
        )
    }
    .map_err(|e| WinError::com("IWbemServices::ExecMethod", &e))
}
