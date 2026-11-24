//! Ventana oculta con bucle de mensajes: la base del hilo del motor.
//!
//! Es de nivel superior (no *message-only*) para recibir los mensajes de
//! difusión que el motor necesita: `WM_DISPLAYCHANGE`, `WM_POWERBROADCAST`,
//! `WM_SETTINGCHANGE` y los avisos de sesión.

use std::ffi::c_void;
use std::sync::atomic::{AtomicIsize, Ordering};

use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::RemoteDesktop::{
    NOTIFY_FOR_THIS_SESSION, WTSRegisterSessionNotification, WTSUnRegisterSessionNotification,
};
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, RegisterHotKey, UnregisterHotKey,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, ChangeWindowMessageFilterEx, CreateWindowExW, DefWindowProcW, DestroyWindow,
    DispatchMessageW, EVENT_SYSTEM_FOREGROUND, GWLP_USERDATA, GetMessageW, GetWindowLongPtrW,
    KillTimer, MSG, MSGFLT_ALLOW, PostMessageW, PostQuitMessage, RegisterClassExW,
    RegisterWindowMessageW, SetTimer, SetWindowLongPtrW, TranslateMessage, WINEVENT_OUTOFCONTEXT,
    WINEVENT_SKIPOWNPROCESS, WM_DESTROY, WM_NCCREATE, WM_NCDESTROY, WNDCLASSEXW, WS_EX_TOOLWINDOW,
    WS_POPUP,
};
use windows::core::{HSTRING, PCWSTR, w};

use crate::{WinError, WinResult};

/// Función que atiende los mensajes de la ventana. Devuelve `Some(resultado)`
/// si lo atendió, o `None` para el procesamiento por defecto.
pub type MessageHandler = Box<dyn FnMut(isize, u32, usize, isize) -> Option<isize>>;

struct WindowState {
    /// Se saca mientras corre, así una llamada reentrante no lo usa dos veces.
    handler: Option<MessageHandler>,
}

/// Ventana oculta. Vive en el hilo que la creó (no es `Send`).
#[derive(Debug)]
pub struct HiddenWindow {
    hwnd: HWND,
}

impl HiddenWindow {
    /// Crea la ventana con la clase `class_name`.
    pub fn create(class_name: &str, handler: MessageHandler) -> WinResult<Self> {
        let class = HSTRING::from(class_name);
        // SAFETY: devuelve el módulo del ejecutable actual.
        let module = unsafe { GetModuleHandleW(PCWSTR::null()) }
            .map_err(|e| WinError::api("GetModuleHandleW", e))?;
        let instance = HINSTANCE(module.0);
        let window_class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: PCWSTR(class.as_ptr()),
            ..Default::default()
        };
        // SAFETY: estructura válida; `class` vive durante la llamada. Si la
        // clase ya existe, se reutiliza.
        unsafe { RegisterClassExW(&window_class) };

        let state = Box::into_raw(Box::new(WindowState {
            handler: Some(handler),
        }));
        // SAFETY: la clase está registrada; la ventana adopta `state` en
        // WM_NCCREATE y lo libera en WM_NCDESTROY.
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                PCWSTR(class.as_ptr()),
                w!("VRes"),
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(instance),
                Some(state as *const c_void),
            )
        }
        .map_err(|e| WinError::api("CreateWindowExW", e))?;
        Ok(Self { hwnd })
    }

    /// Handle como número (para cruzar hilos).
    pub fn handle(&self) -> isize {
        self.hwnd.0 as isize
    }

    /// Deja pasar el mensaje `message` desde procesos con menos privilegios
    /// (por ejemplo, el instalador si VRes corre como administrador).
    pub fn allow_from_lower_integrity(&self, message: u32) {
        // SAFETY: hwnd es una ventana válida de este hilo.
        let _ = unsafe { ChangeWindowMessageFilterEx(self.hwnd, message, MSGFLT_ALLOW, None) };
    }

    /// Programa (o reprograma) el temporizador `id`.
    pub fn set_timer(&self, id: usize, milliseconds: u32) {
        // SAFETY: hwnd es una ventana válida de este hilo; sin callback, llega
        // como WM_TIMER.
        unsafe { SetTimer(Some(self.hwnd), id, milliseconds, None) };
    }

    /// Cancela el temporizador `id`.
    pub fn kill_timer(&self, id: usize) {
        // SAFETY: cancelar un temporizador inexistente no tiene efectos.
        let _ = unsafe { KillTimer(Some(self.hwnd), id) };
    }

    /// Registra un atajo global (`WM_HOTKEY` con `wparam = id`).
    pub fn register_hotkey(&self, id: i32, modifiers: u32, virtual_key: u32) -> WinResult<()> {
        // SAFETY: hwnd es una ventana válida de este hilo.
        unsafe {
            RegisterHotKey(
                Some(self.hwnd),
                id,
                HOT_KEY_MODIFIERS(modifiers),
                virtual_key,
            )
        }
        .map_err(|e| WinError::api("RegisterHotKey", e))
    }

    /// Libera un atajo global.
    pub fn unregister_hotkey(&self, id: i32) {
        // SAFETY: liberar un atajo inexistente no tiene efectos.
        let _ = unsafe { UnregisterHotKey(Some(self.hwnd), id) };
    }

    /// Pide los avisos de bloqueo y desbloqueo de la sesión
    /// (`WM_WTSSESSION_CHANGE`).
    pub fn register_session_notifications(&self) -> bool {
        // SAFETY: hwnd es una ventana válida de este hilo.
        unsafe { WTSRegisterSessionNotification(self.hwnd, NOTIFY_FOR_THIS_SESSION) }.is_ok()
    }
}

impl Drop for HiddenWindow {
    fn drop(&mut self) {
        // SAFETY: se cancela el registro (si lo había) y se destruye la ventana
        // que creamos, una sola vez.
        unsafe {
            let _ = WTSUnRegisterSessionNotification(self.hwnd);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        // SAFETY: en WM_NCCREATE, lparam apunta a un CREATESTRUCTW válido cuyo
        // lpCreateParams es el puntero a WindowState que pasamos al crear.
        unsafe {
            let create = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        }
    }

    // SAFETY: GWLP_USERDATA contiene 0 o el puntero a WindowState adoptado.
    let state = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut WindowState;

    if !state.is_null() {
        if msg == WM_NCDESTROY {
            // SAFETY: último mensaje de la ventana: se libera el estado una vez.
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                drop(Box::from_raw(state));
            }
        } else {
            // SAFETY: el estado es válido hasta WM_NCDESTROY y solo se usa en
            // este hilo. El handler se saca mientras corre (reentrancia).
            let handler = unsafe { (*state).handler.take() };
            if let Some(mut handler) = handler {
                let result = handler(hwnd.0 as isize, msg, wparam.0, lparam.0);
                // SAFETY: el estado sigue vivo (WM_NCDESTROY no pudo ocurrir
                // mientras corría el handler sin pasar por esta rama).
                unsafe { (*state).handler = Some(handler) };
                if let Some(result) = result {
                    return LRESULT(result);
                }
            }
        }
    }

    if msg == WM_DESTROY {
        // SAFETY: termina el bucle de mensajes de este hilo.
        unsafe { PostQuitMessage(0) };
    }
    // SAFETY: procesamiento por defecto con los mismos parámetros.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// Procesa mensajes hasta `WM_QUIT`.
pub fn run_message_loop() {
    let mut message = MSG::default();
    loop {
        // SAFETY: `message` es un MSG válido que vive durante todo el bucle.
        let result = unsafe { GetMessageW(&mut message, None, 0, 0) };
        if result.0 <= 0 {
            break;
        }
        // SAFETY: `message` fue completado por GetMessageW.
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

/// Envía un mensaje a una ventana sin esperar respuesta. Se puede llamar
/// desde cualquier hilo.
pub fn post_message(hwnd: isize, message: u32, wparam: usize, lparam: isize) -> bool {
    // SAFETY: si la ventana ya no existe, PostMessageW falla sin efectos.
    unsafe {
        PostMessageW(
            Some(HWND(hwnd as *mut c_void)),
            message,
            WPARAM(wparam),
            LPARAM(lparam),
        )
    }
    .is_ok()
}

/// Identificador de un mensaje registrado por nombre (igual en todos los
/// procesos).
pub fn register_message(name: &str) -> u32 {
    let name = HSTRING::from(name);
    // SAFETY: `name` es una cadena UTF-16 terminada en NUL que vive durante la llamada.
    unsafe { RegisterWindowMessageW(PCWSTR(name.as_ptr())) }
}

/// Ventana a la que se avisa cada cambio de ventana en primer plano.
static FOREGROUND_TARGET: AtomicIsize = AtomicIsize::new(0);
/// Mensaje con el que se avisa.
static FOREGROUND_MESSAGE: AtomicIsize = AtomicIsize::new(0);

extern "system" fn on_foreground(
    _hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    _object: i32,
    _child: i32,
    _thread: u32,
    _time: u32,
) {
    let target = FOREGROUND_TARGET.load(Ordering::Relaxed);
    let message = FOREGROUND_MESSAGE.load(Ordering::Relaxed);
    if target != 0 && message != 0 {
        post_message(target, message as u32, 0, hwnd.0 as isize);
    }
}

/// Aviso de cambio de ventana en primer plano. **No inyecta nada** en otros
/// procesos (`WINEVENT_OUTOFCONTEXT`): Windows llama a esta función en el hilo
/// que la registró, que tiene que procesar mensajes.
#[derive(Debug)]
pub struct ForegroundHook {
    hook: HWINEVENTHOOK,
}

impl ForegroundHook {
    /// Cada cambio de primer plano llega a `target` como `message`, con la
    /// ventana nueva en `lparam`.
    pub fn install(target: isize, message: u32) -> WinResult<Self> {
        FOREGROUND_TARGET.store(target, Ordering::Relaxed);
        FOREGROUND_MESSAGE.store(message as isize, Ordering::Relaxed);
        // SAFETY: `on_foreground` es una función estática; fuera de contexto
        // no se carga en otros procesos.
        let hook = unsafe {
            SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(on_foreground),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            )
        };
        if hook.is_invalid() {
            Err(WinError::Unsupported("SetWinEventHook"))
        } else {
            Ok(Self { hook })
        }
    }
}

impl Drop for ForegroundHook {
    fn drop(&mut self) {
        FOREGROUND_TARGET.store(0, Ordering::Relaxed);
        // SAFETY: quita el aviso que instalamos, una sola vez.
        let _ = unsafe { UnhookWinEvent(self.hook) };
    }
}

/// Programa (o reprograma) el temporizador `id` de la ventana `hwnd`. Solo
/// desde el hilo de la ventana.
pub fn set_timer(hwnd: isize, id: usize, milliseconds: u32) {
    // SAFETY: si la ventana no existe, SetTimer falla sin efectos.
    unsafe { SetTimer(Some(HWND(hwnd as *mut c_void)), id, milliseconds, None) };
}

/// Cancela el temporizador `id` de la ventana `hwnd`.
pub fn kill_timer(hwnd: isize, id: usize) {
    // SAFETY: cancelar un temporizador inexistente no tiene efectos.
    let _ = unsafe { KillTimer(Some(HWND(hwnd as *mut c_void)), id) };
}

/// Registra un atajo global para la ventana `hwnd`. Solo desde el hilo de la
/// ventana (Windows no admite asociarlo a una ventana de otro hilo).
pub fn register_hotkey(hwnd: isize, id: i32, modifiers: u32, virtual_key: u32) -> WinResult<()> {
    // SAFETY: si la ventana no es de este hilo o no existe, la llamada falla
    // sin efectos.
    unsafe {
        RegisterHotKey(
            Some(HWND(hwnd as *mut c_void)),
            id,
            HOT_KEY_MODIFIERS(modifiers),
            virtual_key,
        )
    }
    .map_err(|e| WinError::api("RegisterHotKey", e))
}

/// Libera un atajo global de la ventana `hwnd`.
pub fn unregister_hotkey(hwnd: isize, id: i32) {
    // SAFETY: liberar un atajo inexistente no tiene efectos.
    let _ = unsafe { UnregisterHotKey(Some(HWND(hwnd as *mut c_void)), id) };
}
