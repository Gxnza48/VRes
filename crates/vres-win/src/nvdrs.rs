//! Perfiles del driver de NVIDIA (DRS de NVAPI): ajustes por juego.
//!
//! Es lo mismo que cambia el panel de control de NVIDIA en el perfil de un
//! ejecutable: no se toca el juego ni su proceso, solo la base de ajustes del
//! driver. Los cambios quedan guardados en el driver hasta que se quitan, y el
//! juego los toma la próxima vez que se abre.
//!
//! IDs de funciones y estructuras: `nvapi.h` y `nvapi_interface.h` de NVIDIA
//! (MIT); ver `THIRD-PARTY-NOTICES.md`.

use std::ffi::c_void;
use std::ptr::null_mut;

use windows::Win32::Foundation::{FreeLibrary, HMODULE};
use windows::core::w;

use crate::vibrance::{driver_error, load_system_dll, symbol, typed, typed_optional};
use crate::{WinError, WinResult};

const NVAPI_OK: i32 = 0;
/// El ajuste no está puesto en ese perfil.
const SETTING_NOT_FOUND: i32 = -160;
/// El ejecutable no está en ningún perfil.
const EXECUTABLE_NOT_FOUND: i32 = -166;
/// Faltan permisos (hace falta administrador).
pub const INVALID_USER_PRIVILEGE: i32 = -137;

const NV_INITIALIZE: u32 = 0x0150_E828;
const NV_UNLOAD: u32 = 0xD22B_DD7E;
const DRS_CREATE_SESSION: u32 = 0x0694_D52E;
const DRS_DESTROY_SESSION: u32 = 0xDAD9_CFF8;
const DRS_LOAD_SETTINGS: u32 = 0x375D_BD6B;
const DRS_SAVE_SETTINGS: u32 = 0xFCBC_7E14;
const DRS_FIND_APPLICATION_BY_NAME: u32 = 0xEEE5_66B2;
const DRS_FIND_PROFILE_BY_NAME: u32 = 0x7E4A_9A0B;
const DRS_CREATE_PROFILE: u32 = 0xCC17_6068;
const DRS_CREATE_APPLICATION: u32 = 0x4347_A9DE;
const DRS_GET_SETTING: u32 = 0x73BF_8338;
const DRS_SET_SETTING: u32 = 0x577D_D202;
const DRS_DELETE_PROFILE_SETTING: u32 = 0xE4A2_6362;
const SYS_GET_DRIVER_AND_BRANCH_VERSION: u32 = 0x2926_AAAD;

const UNICODE_LEN: usize = 2048;
type UnicodeString = [u16; UNICODE_LEN];

/// `NVDRS_SETTING_V1` (12 320 bytes). Las dos uniones (valor predefinido y
/// actual) miden 4100 bytes; el valor DWORD es su primer `u32`.
#[repr(C)]
struct DrsSetting {
    version: u32,
    setting_name: UnicodeString,
    setting_id: u32,
    setting_type: u32,
    setting_location: u32,
    is_current_predefined: u32,
    is_predefined_valid: u32,
    predefined: [u32; 1025],
    current: [u32; 1025],
}

/// `NVDRS_APPLICATION_V4` (20 492 bytes).
#[repr(C)]
struct DrsApplication {
    version: u32,
    is_predefined: u32,
    app_name: UnicodeString,
    user_friendly_name: UnicodeString,
    launcher: UnicodeString,
    file_in_folder: UnicodeString,
    flags: u32,
    command_line: UnicodeString,
}

/// `NVDRS_PROFILE_V1` (4116 bytes).
#[repr(C)]
struct DrsProfile {
    version: u32,
    profile_name: UnicodeString,
    gpu_support: u32,
    is_predefined: u32,
    num_of_apps: u32,
    num_of_settings: u32,
}

const _: () = assert!(size_of::<DrsSetting>() == 12320);
const _: () = assert!(size_of::<DrsApplication>() == 20492);
const _: () = assert!(size_of::<DrsProfile>() == 4116);
const SETTING_VERSION: u32 = size_of::<DrsSetting>() as u32 | (1 << 16);
const APPLICATION_VERSION: u32 = size_of::<DrsApplication>() as u32 | (4 << 16);
const PROFILE_VERSION: u32 = size_of::<DrsProfile>() as u32 | (1 << 16);
/// Tipo de valor `NVDRS_DWORD_TYPE`.
const DWORD_TYPE: u32 = 0;
/// El ajuste está en el perfil del juego (no heredado del global).
const CURRENT_PROFILE_LOCATION: u32 = 0;

type Handle = *mut c_void;
type NoArgs = unsafe extern "C" fn() -> i32;
type QueryInterface = unsafe extern "C" fn(u32) -> *mut c_void;
type CreateSession = unsafe extern "C" fn(*mut Handle) -> i32;
type SessionOnly = unsafe extern "C" fn(Handle) -> i32;
type FindApplication =
    unsafe extern "C" fn(Handle, *const u16, *mut Handle, *mut DrsApplication) -> i32;
type FindProfile = unsafe extern "C" fn(Handle, *const u16, *mut Handle) -> i32;
type CreateProfile = unsafe extern "C" fn(Handle, *mut DrsProfile, *mut Handle) -> i32;
type CreateApplication = unsafe extern "C" fn(Handle, Handle, *mut DrsApplication) -> i32;
type GetSetting = unsafe extern "C" fn(Handle, Handle, u32, *mut DrsSetting) -> i32;
type SetSetting = unsafe extern "C" fn(Handle, Handle, *mut DrsSetting) -> i32;
type DeleteSetting = unsafe extern "C" fn(Handle, Handle, u32) -> i32;
type DriverVersion = unsafe extern "C" fn(*mut u32, *mut [u8; 64]) -> i32;

/// Valor de un ajuste en el perfil de un juego.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingValue {
    pub id: u32,
    /// `None`: el perfil no lo tiene puesto (vale el del driver).
    pub value: Option<u32>,
}

fn unicode(text: &str) -> Box<UnicodeString> {
    let mut buffer = Box::new([0u16; UNICODE_LEN]);
    for (slot, unit) in buffer
        .iter_mut()
        .zip(text.encode_utf16().take(UNICODE_LEN - 1))
    {
        *slot = unit;
    }
    buffer
}

fn zeroed<T>() -> Box<T> {
    let layout = std::alloc::Layout::new::<T>();
    // SAFETY: solo se usa con las estructuras de DRS de este archivo (tamaño
    // no nulo y solo enteros: todo en cero es un valor válido). La memoria se
    // pide con el layout de `T`, así `Box` la libera bien.
    unsafe {
        let pointer = std::alloc::alloc_zeroed(layout).cast::<T>();
        if pointer.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        Box::from_raw(pointer)
    }
}

fn check(api: &'static str, status: i32) -> WinResult<()> {
    if status == NVAPI_OK {
        Ok(())
    } else {
        Err(driver_error(api, status))
    }
}

struct Api {
    module: HMODULE,
    unload: NoArgs,
    create_session: CreateSession,
    destroy_session: SessionOnly,
    load_settings: SessionOnly,
    save_settings: SessionOnly,
    find_application: FindApplication,
    find_profile: FindProfile,
    create_profile: CreateProfile,
    create_application: CreateApplication,
    get_setting: GetSetting,
    set_setting: SetSetting,
    delete_setting: DeleteSetting,
    driver_version: Option<DriverVersion>,
}

impl Api {
    fn load() -> WinResult<Self> {
        let module = load_system_dll(w!("nvapi64.dll")).ok_or(WinError::Unsupported("nvapi"))?;
        match Self::bind(module) {
            Some(api) => Ok(api),
            None => {
                // SAFETY: el módulo es nuestro y nadie más lo usa.
                let _ = unsafe { FreeLibrary(module) };
                Err(WinError::Unsupported("nvapi drs"))
            }
        }
    }

    fn bind(module: HMODULE) -> Option<Self> {
        let query = symbol(module, b"nvapi_QueryInterface\0")?;
        // SAFETY: `nvapi_QueryInterface` tiene esta firma en todas las versiones de NVAPI.
        let query: QueryInterface = unsafe { typed(query) };
        let function = |id: u32| -> Option<*const c_void> {
            // SAFETY: devuelve un puntero a función o null si no existe.
            let pointer = unsafe { query(id) };
            (!pointer.is_null()).then_some(pointer.cast_const())
        };
        // SAFETY: (vale para todas las conversiones de acá) cada ID corresponde
        // a la firma declarada en su tipo, según `nvapi.h`.
        let initialize: NoArgs = unsafe { typed(function(NV_INITIALIZE)?) };
        // SAFETY: inicializa la biblioteca (con conteo de referencias).
        if unsafe { initialize() } != NVAPI_OK {
            return None;
        }
        // SAFETY: ver arriba.
        unsafe {
            Some(Self {
                module,
                unload: typed(function(NV_UNLOAD)?),
                create_session: typed(function(DRS_CREATE_SESSION)?),
                destroy_session: typed(function(DRS_DESTROY_SESSION)?),
                load_settings: typed(function(DRS_LOAD_SETTINGS)?),
                save_settings: typed(function(DRS_SAVE_SETTINGS)?),
                find_application: typed(function(DRS_FIND_APPLICATION_BY_NAME)?),
                find_profile: typed(function(DRS_FIND_PROFILE_BY_NAME)?),
                create_profile: typed(function(DRS_CREATE_PROFILE)?),
                create_application: typed(function(DRS_CREATE_APPLICATION)?),
                get_setting: typed(function(DRS_GET_SETTING)?),
                set_setting: typed(function(DRS_SET_SETTING)?),
                delete_setting: typed(function(DRS_DELETE_PROFILE_SETTING)?),
                driver_version: typed_optional(function(SYS_GET_DRIVER_AND_BRANCH_VERSION)),
            })
        }
    }
}

impl Drop for Api {
    fn drop(&mut self) {
        // SAFETY: descuenta la inicialización de `bind` y libera nuestro módulo.
        unsafe {
            (self.unload)();
            let _ = FreeLibrary(self.module);
        }
    }
}

/// Una sesión de DRS con los ajustes del driver cargados.
struct Session {
    api: Api,
    handle: Handle,
}

impl Session {
    fn open() -> WinResult<Self> {
        let api = Api::load()?;
        let mut handle = null_mut();
        // SAFETY: `handle` vive durante la llamada.
        check("NvAPI_DRS_CreateSession", unsafe {
            (api.create_session)(&mut handle)
        })?;
        let session = Self { api, handle };
        // SAFETY: sesión recién creada.
        check("NvAPI_DRS_LoadSettings", unsafe {
            (session.api.load_settings)(session.handle)
        })?;
        Ok(session)
    }

    /// El perfil que tiene al ejecutable, si hay uno.
    fn find_application(&self, executable: &str) -> WinResult<Option<Handle>> {
        let name = unicode(executable);
        let mut profile = null_mut();
        let mut application = zeroed::<DrsApplication>();
        application.version = APPLICATION_VERSION;
        // SAFETY: cadena terminada en cero y estructura con su versión, vivas durante la llamada.
        let status = unsafe {
            (self.api.find_application)(self.handle, name.as_ptr(), &mut profile, &mut *application)
        };
        match status {
            NVAPI_OK => Ok(Some(profile)),
            EXECUTABLE_NOT_FOUND => Ok(None),
            code => Err(driver_error("NvAPI_DRS_FindApplicationByName", code)),
        }
    }

    /// El perfil del ejecutable; si no tiene, crea uno propio con ese nombre.
    fn profile_for(&self, executable: &str, profile_name: &str) -> WinResult<Handle> {
        if let Some(profile) = self.find_application(executable)? {
            return Ok(profile);
        }
        let name = unicode(profile_name);
        let mut profile = null_mut();
        // SAFETY: cadena terminada en cero; `profile` vive durante la llamada.
        let found = unsafe { (self.api.find_profile)(self.handle, name.as_ptr(), &mut profile) };
        if found != NVAPI_OK {
            let mut info = zeroed::<DrsProfile>();
            info.version = PROFILE_VERSION;
            info.profile_name = *name;
            // SAFETY: estructura con su versión; `profile` vive durante la llamada.
            check("NvAPI_DRS_CreateProfile", unsafe {
                (self.api.create_profile)(self.handle, &mut *info, &mut profile)
            })?;
        }
        let mut application = zeroed::<DrsApplication>();
        application.version = APPLICATION_VERSION;
        application.app_name = *unicode(executable);
        // SAFETY: estructura con su versión, viva durante la llamada.
        check("NvAPI_DRS_CreateApplication", unsafe {
            (self.api.create_application)(self.handle, profile, &mut *application)
        })?;
        Ok(profile)
    }

    /// Valor propio del perfil (`None` si no lo tiene puesto o lo hereda).
    fn get(&self, profile: Handle, id: u32) -> WinResult<Option<u32>> {
        let mut setting = zeroed::<DrsSetting>();
        setting.version = SETTING_VERSION;
        // SAFETY: estructura con su versión, viva durante la llamada.
        let status = unsafe { (self.api.get_setting)(self.handle, profile, id, &mut *setting) };
        match status {
            NVAPI_OK => Ok((setting.setting_location == CURRENT_PROFILE_LOCATION
                && setting.setting_type == DWORD_TYPE)
                .then_some(setting.current[0])),
            SETTING_NOT_FOUND => Ok(None),
            code => Err(driver_error("NvAPI_DRS_GetSetting", code)),
        }
    }

    fn set(&self, profile: Handle, id: u32, value: u32) -> WinResult<()> {
        let mut setting = zeroed::<DrsSetting>();
        setting.version = SETTING_VERSION;
        setting.setting_id = id;
        setting.setting_type = DWORD_TYPE;
        setting.current[0] = value;
        // SAFETY: estructura con su versión, viva durante la llamada.
        check("NvAPI_DRS_SetSetting", unsafe {
            (self.api.set_setting)(self.handle, profile, &mut *setting)
        })
    }

    fn delete(&self, profile: Handle, id: u32) -> WinResult<()> {
        // SAFETY: handles válidos de esta sesión.
        match unsafe { (self.api.delete_setting)(self.handle, profile, id) } {
            NVAPI_OK | SETTING_NOT_FOUND => Ok(()),
            code => Err(driver_error("NvAPI_DRS_DeleteProfileSetting", code)),
        }
    }

    fn save(&self) -> WinResult<()> {
        // SAFETY: sesión válida.
        check("NvAPI_DRS_SaveSettings", unsafe {
            (self.api.save_settings)(self.handle)
        })
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // SAFETY: la sesión es nuestra y no se usa después.
        unsafe { (self.api.destroy_session)(self.handle) };
    }
}

/// `true` si hay driver de NVIDIA con perfiles (DRS).
pub fn available() -> bool {
    Session::open().is_ok()
}

/// Versión del driver en centésimas (475.14 = 47514), si se puede leer.
pub fn driver_version() -> Option<u32> {
    let api = Api::load().ok()?;
    let get = api.driver_version?;
    let mut version = 0u32;
    let mut branch = [0u8; 64];
    // SAFETY: ambos punteros viven durante la llamada; la rama mide 64 bytes.
    let status = unsafe { get(&mut version, &mut branch) };
    (status == NVAPI_OK && version > 0).then_some(version)
}

/// Lee los ajustes `ids` del perfil del ejecutable (sin cambiar nada).
pub fn read(executable: &str, ids: &[u32]) -> WinResult<Vec<SettingValue>> {
    let session = Session::open()?;
    let Some(profile) = session.find_application(executable)? else {
        return Ok(ids
            .iter()
            .map(|&id| SettingValue { id, value: None })
            .collect());
    };
    ids.iter()
        .map(|&id| {
            Ok(SettingValue {
                id,
                value: session.get(profile, id)?,
            })
        })
        .collect()
}

/// Escribe los valores en el perfil del ejecutable (lo crea si hace falta) y
/// los guarda en el driver. `None` quita el ajuste (vuelve al del driver).
/// Devuelve los valores que había antes, para poder volver a ellos.
pub fn write(
    executable: &str,
    profile_name: &str,
    values: &[SettingValue],
) -> WinResult<Vec<SettingValue>> {
    let session = Session::open()?;
    let profile = session.profile_for(executable, profile_name)?;
    let mut before = Vec::with_capacity(values.len());
    for setting in values {
        before.push(SettingValue {
            id: setting.id,
            value: session.get(profile, setting.id)?,
        });
        match setting.value {
            Some(value) => session.set(profile, setting.id, value)?,
            None => session.delete(profile, setting.id)?,
        }
    }
    session.save()?;
    Ok(before)
}

/// `true` si el error es por falta de permisos de administrador.
pub fn needs_admin(error: &WinError) -> bool {
    matches!(error, WinError::DisplayChange { code, .. } if *code == INVALID_USER_PRIVILEGE)
}

/// Estado del potato de un juego (solo lee).
pub fn potato_status(game: &vres_core::potato::PotatoGame) -> vres_core::potato::PotatoStatus {
    use vres_core::potato::{ALL_SETTINGS, PotatoLevel, PotatoStatus, detect};
    match read(&game.executable, &ALL_SETTINGS) {
        Ok(values) => {
            let pairs: Vec<(u32, Option<u32>)> = values.iter().map(|v| (v.id, v.value)).collect();
            let (level, custom) = detect(&pairs);
            PotatoStatus {
                available: true,
                game: game.clone(),
                level,
                custom,
                legacy: driver_version().is_some_and(vres_core::potato::legacy_driver),
            }
        }
        Err(_) => PotatoStatus {
            available: false,
            game: game.clone(),
            level: PotatoLevel::Off,
            custom: false,
            legacy: false,
        },
    }
}

/// Pone un nivel de potato en el perfil del juego y lo guarda en el driver.
pub fn potato_apply(
    game: &vres_core::potato::PotatoGame,
    level: vres_core::potato::PotatoLevel,
) -> WinResult<()> {
    use vres_core::potato::{PROFILE_NAME, values};
    let wanted: Vec<SettingValue> = values(level)
        .into_iter()
        .map(|(id, value)| SettingValue { id, value })
        .collect();
    write(&game.executable, PROFILE_NAME, &wanted).map(|_| ())
}
