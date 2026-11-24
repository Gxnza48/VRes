//! Ventana de control de VRes.
//!
//! VRes corre una ventana oculta de nivel superior con la clase
//! [`WINDOW_CLASS`]. Otros procesos —`vres.exe --shutdown` y `--reset`,
//! `vres-cli` y el instalador— la encuentran con `FindWindowW` y le mandan un
//! mensaje registrado ([`RemoteCommand`]).
//!
//! Es de nivel superior y no *message-only* a propósito: las ventanas
//! *message-only* no reciben mensajes de difusión como `WM_DISPLAYCHANGE` o
//! `WM_POWERBROADCAST`, que el motor de color necesita (fase F1).

use std::ffi::c_void;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{
    CloseHandle, HINSTANCE, HWND, LPARAM, LRESULT, WAIT_OBJECT_0, WPARAM,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject};
use windows::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, ChangeWindowMessageFilterEx, CreateWindowExW, DefWindowProcW, DispatchMessageW,
    FindWindowW, GWLP_USERDATA, GetMessageW, GetWindowLongPtrW, GetWindowThreadProcessId, MSG,
    MSGFLT_ALLOW, PostMessageW, PostQuitMessage, RegisterClassExW, RegisterWindowMessageW,
    SMTO_ABORTIFHUNG, SMTO_BLOCK, SendMessageTimeoutW, SetWindowLongPtrW, TranslateMessage,
    WM_CLOSE, WM_DESTROY, WM_NCCREATE, WM_NCDESTROY, WNDCLASSEXW, WS_EX_TOOLWINDOW, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use crate::{WinError, WinResult};

/// Clase de la ventana de control.
pub const WINDOW_CLASS: PCWSTR = w!("VRes.Engine");
/// Clase de la ventana de control, como texto.
pub const WINDOW_CLASS_NAME: &str = "VRes.Engine";
/// Nombre del mensaje registrado que transporta los [`RemoteCommand`].
pub const REMOTE_MESSAGE_NAME: &str = "VRes.Remote.v1";

/// Nombre del mensaje registrado que transporta los [`RemoteCommand`].
const REMOTE_MESSAGE: PCWSTR = w!("VRes.Remote.v1");

/// Pedidos que otro proceso le puede hacer a la instancia en ejecución.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum RemoteCommand {
    /// Volver la pantalla a la normalidad (la app sigue abierta).
    EmergencyReset = 1,
    /// Cerrar VRes restaurando todo.
    Shutdown = 2,
    /// Comprobar que la instancia responde.
    Ping = 3,
}

impl RemoteCommand {
    /// Interpreta el `wparam` del mensaje remoto.
    pub fn from_wparam(value: usize) -> Option<Self> {
        match value {
            1 => Some(Self::EmergencyReset),
            2 => Some(Self::Shutdown),
            3 => Some(Self::Ping),
            _ => None,
        }
    }
}

/// Valor que la ventana devuelve cuando recibió y aceptó un pedido.
pub const ACK: isize = 1;

fn remote_message_id() -> WinResult<u32> {
    // SAFETY: REMOTE_MESSAGE es una cadena UTF-16 estática terminada en NUL.
    let id = unsafe { RegisterWindowMessageW(REMOTE_MESSAGE) };
    if id == 0 {
        Err(WinError::last("RegisterWindowMessageW"))
    } else {
        Ok(id)
    }
}

/// Busca la ventana de control de una instancia de VRes en ejecución.
fn find_running_window() -> Option<HWND> {
    // SAFETY: la clase es una cadena estática terminada en NUL y el título se
    // pasa como nulo (no se filtra por título).
    let found = unsafe { FindWindowW(WINDOW_CLASS, PCWSTR::null()) };
    found.ok().filter(|hwnd| !hwnd.is_invalid())
}

/// `true` si hay una instancia de VRes en ejecución.
pub fn is_running() -> bool {
    find_running_window().is_some()
}

/// Le envía un pedido a la instancia en ejecución y espera la respuesta.
///
/// Devuelve `Ok(None)` si VRes no está corriendo, u `Ok(Some(respuesta))`.
pub fn send(command: RemoteCommand, timeout: Duration) -> WinResult<Option<isize>> {
    match find_running_window() {
        Some(hwnd) => send_to(hwnd, command, timeout).map(Some),
        None => Ok(None),
    }
}

fn send_to(hwnd: HWND, command: RemoteCommand, timeout: Duration) -> WinResult<isize> {
    let message = remote_message_id()?;
    let timeout_ms = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX);
    let mut result: usize = 0;
    // SAFETY: si la ventana desaparece entre medio, SendMessageTimeoutW falla
    // de forma controlada. `result` vive durante toda la llamada.
    let sent = unsafe {
        SendMessageTimeoutW(
            hwnd,
            message,
            WPARAM(command as usize),
            LPARAM(0),
            SMTO_ABORTIFHUNG | SMTO_BLOCK,
            timeout_ms,
            Some(&mut result as *mut usize),
        )
    };
    if sent.0 == 0 {
        return Err(WinError::Timeout("SendMessageTimeoutW"));
    }
    // El resultado es un LRESULT transportado como usize: se reinterpreta.
    Ok(result as isize)
}

/// Resultado de pedirle a VRes que se cierre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownOutcome {
    /// No había ninguna instancia en ejecución.
    NotRunning,
    /// La instancia se cerró dentro del tiempo de espera.
    Exited,
    /// La instancia sigue viva al vencer el tiempo de espera.
    StillRunning,
}

/// Le pide a la instancia en ejecución que se cierre y espera a que su
/// **proceso** termine (no solo su ventana), con un tope de `timeout`.
///
/// Lo usan `vres.exe --shutdown` y el instalador antes de reemplazar archivos.
/// Solo abre un handle de sincronización al proceso de VRes, nunca a otro.
pub fn shutdown_running(timeout: Duration) -> WinResult<ShutdownOutcome> {
    let Some(hwnd) = find_running_window() else {
        return Ok(ShutdownOutcome::NotRunning);
    };
    let start = Instant::now();

    let mut pid: u32 = 0;
    // SAFETY: hwnd viene de FindWindowW y `pid` vive durante la llamada.
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid as *mut u32)) };
    let process = if pid == 0 {
        None
    } else {
        // SAFETY: es el proceso de VRes (dueño de la ventana de control) y
        // solo se pide permiso de sincronización: esperar a que termine.
        unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid) }.ok()
    };

    send_to(hwnd, RemoteCommand::Shutdown, timeout)?;

    let remaining = timeout.saturating_sub(start.elapsed());
    let exited = match process {
        Some(handle) => {
            let millis = u32::try_from(remaining.as_millis()).unwrap_or(u32::MAX);
            // SAFETY: `handle` es un handle válido que abrimos arriba.
            let wait = unsafe { WaitForSingleObject(handle, millis) };
            // SAFETY: se cierra exactamente una vez el handle que abrimos.
            let _ = unsafe { CloseHandle(handle) };
            wait == WAIT_OBJECT_0
        }
        // Sin handle (no debería pasar): se espera a que desaparezca la ventana.
        None => {
            let deadline = Instant::now() + remaining;
            while is_running() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(50));
            }
            !is_running()
        }
    };
    Ok(if exited {
        ShutdownOutcome::Exited
    } else {
        ShutdownOutcome::StillRunning
    })
}

/// Función que atiende los pedidos remotos. Corre en el hilo de la ventana.
pub type RemoteHandler = Box<dyn Fn(RemoteCommand) -> isize + Send + 'static>;

/// Estado guardado en la ventana (`GWLP_USERDATA`).
struct WindowState {
    remote_message: u32,
    handler: RemoteHandler,
}

/// La ventana de control en ejecución, con su hilo.
#[derive(Debug)]
pub struct ControlWindow {
    hwnd: isize,
    thread: Option<JoinHandle<()>>,
}

impl ControlWindow {
    /// Crea la ventana de control en un hilo propio con su bucle de mensajes.
    ///
    /// `handler` recibe cada [`RemoteCommand`] y devuelve la respuesta para el
    /// proceso que lo envió (por convención, [`ACK`]).
    pub fn spawn(handler: RemoteHandler) -> WinResult<Self> {
        let (ready_tx, ready_rx) = mpsc::channel::<WinResult<isize>>();
        let thread = std::thread::Builder::new()
            .name("vres-control".into())
            .spawn(move || match create_window(handler) {
                Ok(hwnd) => {
                    // HWND no es Send: cruza el canal como número.
                    let _ = ready_tx.send(Ok(hwnd.0 as isize));
                    pump_messages();
                }
                Err(error) => {
                    let _ = ready_tx.send(Err(error));
                }
            })
            .map_err(|_| WinError::ThreadStopped("vres-control"))?;

        let hwnd = ready_rx
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| WinError::ThreadStopped("vres-control"))??;
        Ok(Self {
            hwnd,
            thread: Some(thread),
        })
    }

    /// Cierra la ventana y espera a que termine su hilo.
    pub fn close(mut self) {
        self.close_inner();
    }

    fn close_inner(&mut self) {
        let Some(thread) = self.thread.take() else {
            return;
        };
        let hwnd = HWND(self.hwnd as *mut c_void);
        // SAFETY: hwnd es la ventana que creó nuestro hilo y sigue viva mientras
        // el hilo no termine. Si ya no existe, PostMessageW falla sin efectos.
        let posted = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        if posted.is_ok() {
            let _ = thread.join();
        }
    }
}

impl Drop for ControlWindow {
    fn drop(&mut self) {
        self.close_inner();
    }
}

fn create_window(handler: RemoteHandler) -> WinResult<HWND> {
    let remote_message = remote_message_id()?;

    // SAFETY: GetModuleHandleW(None) devuelve el módulo del ejecutable actual.
    let module = unsafe { GetModuleHandleW(PCWSTR::null()) }
        .map_err(|e| WinError::api("GetModuleHandleW", e))?;
    let instance = HINSTANCE(module.0);

    let class = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(window_proc),
        hInstance: instance,
        lpszClassName: WINDOW_CLASS,
        ..Default::default()
    };
    // SAFETY: `class` es una estructura válida y la cadena de la clase es
    // estática. Si la clase ya existe (segunda ventana en el mismo proceso),
    // RegisterClassExW devuelve 0 y CreateWindowExW usa la existente.
    unsafe { RegisterClassExW(&class) };

    let state = Box::into_raw(Box::new(WindowState {
        remote_message,
        handler,
    }));

    // SAFETY: la clase quedó registrada; `state` es un puntero propio que la
    // ventana adopta en WM_NCCREATE y libera en WM_NCDESTROY.
    let created = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            WINDOW_CLASS,
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
    };
    let hwnd = created.map_err(|e| WinError::api("CreateWindowExW", e))?;
    // Si CreateWindowExW falla, `state` se pierde a propósito: no se puede saber
    // si la ventana llegó a adoptarlo. Es un único bloque chico y solo ante error.

    // Si VRes corre como administrador, dejar pasar el mensaje remoto desde
    // procesos sin elevar (por ejemplo, el instalador o la CLI).
    // SAFETY: hwnd es una ventana válida recién creada en este hilo.
    let _ = unsafe { ChangeWindowMessageFilterEx(hwnd, remote_message, MSGFLT_ALLOW, None) };

    Ok(hwnd)
}

fn pump_messages() {
    let mut message = MSG::default();
    loop {
        // SAFETY: `message` es un MSG válido que vive durante todo el bucle.
        let result = unsafe { GetMessageW(&mut message, None, 0, 0) };
        // 0 = WM_QUIT, -1 = error: en ambos casos se termina.
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

extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        // SAFETY: en WM_NCCREATE, lparam apunta a un CREATESTRUCTW válido cuyo
        // lpCreateParams es el puntero a WindowState que pasamos al crear.
        unsafe {
            let create = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        }
    }

    // SAFETY: GWLP_USERDATA contiene 0 o el puntero a WindowState que adoptamos.
    let state = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut WindowState;

    if !state.is_null() {
        // SAFETY: el puntero es válido desde WM_NCCREATE hasta WM_NCDESTROY y
        // solo se usa desde el hilo de esta ventana.
        let state_ref = unsafe { &*state };
        if msg == state_ref.remote_message {
            return match RemoteCommand::from_wparam(wparam.0) {
                Some(command) => LRESULT((state_ref.handler)(command)),
                None => LRESULT(0),
            };
        }
        if msg == WM_NCDESTROY {
            // SAFETY: es el último mensaje de la ventana: se desvincula el
            // estado y se libera exactamente una vez.
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                drop(Box::from_raw(state));
            }
        }
    }

    if msg == WM_DESTROY {
        // SAFETY: termina el bucle de mensajes de este hilo.
        unsafe { PostQuitMessage(0) };
    }

    // SAFETY: procesamiento por defecto con los mismos parámetros recibidos.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn remote_commands_round_trip() {
        for command in [
            RemoteCommand::EmergencyReset,
            RemoteCommand::Shutdown,
            RemoteCommand::Ping,
        ] {
            assert_eq!(RemoteCommand::from_wparam(command as usize), Some(command));
        }
        assert_eq!(RemoteCommand::from_wparam(0), None);
        assert_eq!(RemoteCommand::from_wparam(99), None);
    }

    /// Crea la ventana, le manda un ping desde este mismo proceso y la cierra.
    ///
    /// Se le habla directo a la ventana del test (no con `send`, que buscaría
    /// por clase y podría encontrar una instancia real de VRes abierta).
    #[test]
    fn control_window_answers_ping() {
        let pings = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&pings);
        let window = ControlWindow::spawn(Box::new(move |command| {
            if command == RemoteCommand::Ping {
                counter.fetch_add(1, Ordering::SeqCst);
            }
            ACK
        }))
        .expect("la ventana de control se crea");

        let hwnd = HWND(window.hwnd as *mut c_void);
        let answer = send_to(hwnd, RemoteCommand::Ping, Duration::from_secs(2)).expect("send");
        assert_eq!(answer, ACK);
        assert_eq!(pings.load(Ordering::SeqCst), 1);

        window.close();
    }
}
