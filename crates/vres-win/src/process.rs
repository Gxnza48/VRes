//! Procesos: qué programa está en primer plano y si un juego sigue abierto,
//! **sin abrir nunca el proceso de un juego**.
//!
//! El nombre del ejecutable sale de una foto de la lista de procesos
//! (`CreateToolhelp32Snapshot`), que no pide permisos sobre ningún proceso.

use std::collections::HashMap;

use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};

use crate::{WinError, WinResult};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GetForegroundWindow, GetWindow, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, IsWindowVisible,
};
use windows::core::BOOL;

/// Un proceso, identificado por su pid y su ejecutable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessInfo {
    pub pid: u32,
    /// Nombre del ejecutable, por ejemplo `cs2.exe`.
    pub exe: String,
}

/// Un programa con ventana visible (para elegir un juego propio).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowedApp {
    pub exe: String,
    pub title: String,
}

/// Nombre del ejecutable de cada proceso (pid → nombre).
pub fn process_names() -> HashMap<u32, String> {
    let mut names = HashMap::new();
    // SAFETY: foto de la lista de procesos; el handle se cierra abajo.
    let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
        return names;
    };
    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    // SAFETY: `entry` tiene su tamaño en `dwSize` y vive durante el recorrido.
    let mut more = unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok();
    while more {
        let end = entry
            .szExeFile
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(entry.szExeFile.len());
        names.insert(
            entry.th32ProcessID,
            String::from_utf16_lossy(&entry.szExeFile[..end]),
        );
        // SAFETY: ídem.
        more = unsafe { Process32NextW(snapshot, &mut entry) }.is_ok();
    }
    // SAFETY: handle de la foto, cerrado una sola vez.
    let _ = unsafe { CloseHandle(snapshot) };
    names
}

fn window_pid(hwnd: HWND) -> u32 {
    let mut pid = 0u32;
    // SAFETY: solo lee el pid dueño de la ventana; no abre el proceso.
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    pid
}

/// El programa de la ventana en primer plano.
pub fn foreground_process() -> Option<ProcessInfo> {
    // SAFETY: consulta sin efectos.
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        return None;
    }
    let pid = window_pid(hwnd);
    if pid == 0 {
        return None;
    }
    process_names()
        .remove(&pid)
        .map(|exe| ProcessInfo { pid, exe })
}

/// `true` si ese proceso sigue abierto (mismo pid y mismo ejecutable, para no
/// confundirlo con otro que reutilice el pid).
pub fn is_running(process: &ProcessInfo) -> bool {
    process_names()
        .get(&process.pid)
        .is_some_and(|exe| exe.eq_ignore_ascii_case(&process.exe))
}

struct Collected {
    windows: Vec<(u32, String)>,
}

unsafe extern "system" fn collect_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: `lparam` apunta al `Collected` de `windowed_apps`, vivo mientras
    // dura EnumWindows.
    let collected = unsafe { &mut *(lparam.0 as *mut Collected) };
    // SAFETY: consultas de solo lectura sobre una ventana que existe.
    let visible = unsafe { IsWindowVisible(hwnd) }.as_bool();
    // SAFETY: ídem; las ventanas con dueño (diálogos) se saltean.
    let owned = unsafe { GetWindow(hwnd, GW_OWNER) }.is_ok_and(|owner| !owner.is_invalid());
    // SAFETY: ídem.
    let length = unsafe { GetWindowTextLengthW(hwnd) };
    if visible && !owned && length > 0 {
        let mut buffer = vec![0u16; length as usize + 1];
        // SAFETY: el búfer tiene lugar para el título y el cero final.
        let copied = unsafe { GetWindowTextW(hwnd, &mut buffer) };
        let title = String::from_utf16_lossy(&buffer[..copied.max(0) as usize]);
        collected.windows.push((window_pid(hwnd), title));
    }
    BOOL::from(true)
}

/// Programas con ventana visible, sin repetir ejecutables ni incluir a VRes.
pub fn windowed_apps() -> Vec<WindowedApp> {
    let mut collected = Collected {
        windows: Vec::new(),
    };
    // SAFETY: el callback solo usa `collected` durante la llamada.
    let _ = unsafe {
        EnumWindows(
            Some(collect_window),
            LPARAM(&mut collected as *mut Collected as isize),
        )
    };
    let names = process_names();
    let own = std::process::id();
    let mut apps: Vec<WindowedApp> = Vec::new();
    for (pid, title) in collected.windows {
        if pid == own {
            continue;
        }
        let Some(exe) = names.get(&pid) else {
            continue;
        };
        let lower = exe.to_lowercase();
        if matches!(
            lower.as_str(),
            "explorer.exe"
                | "applicationframehost.exe"
                | "textinputhost.exe"
                | "systemsettings.exe"
        ) || apps.iter().any(|app| app.exe.eq_ignore_ascii_case(exe))
        {
            continue;
        }
        apps.push(WindowedApp {
            exe: exe.clone(),
            title,
        });
    }
    apps.sort_by_key(|app| app.title.to_lowercase());
    apps
}

/// `true` si una app Direct3D está en pantalla completa exclusiva (en esa
/// situación el filtro de color de Windows no se ve).
pub fn exclusive_fullscreen() -> bool {
    use windows::Win32::UI::Shell::{QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState};
    // SAFETY: la función solo escribe el estado en el valor que devuelve.
    unsafe { SHQueryUserNotificationState() }
        .is_ok_and(|state| state == QUNS_RUNNING_D3D_FULL_SCREEN)
}

/// Vuelve a abrir este mismo ejecutable **como administrador** con `args`
/// (Windows muestra el aviso de UAC), espera a que termine y devuelve su
/// código de salida. `Ok(None)` si el usuario dijo que no.
pub fn run_self_elevated(args: &str, timeout: std::time::Duration) -> WinResult<Option<u32>> {
    use windows::Win32::Foundation::{ERROR_CANCELLED, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
    use windows::Win32::UI::Shell::{
        SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW,
    };
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
    use windows::core::{HSTRING, PCWSTR, w};

    let exe = std::env::current_exe().map_err(|_| WinError::Unsupported("current_exe"))?;
    let file = HSTRING::from(exe.as_os_str());
    let parameters = HSTRING::from(args);
    let mut info = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
        lpVerb: w!("runas"),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(parameters.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    // SAFETY: las cadenas viven hasta el final de la función y `info` tiene su tamaño.
    if let Err(error) = unsafe { ShellExecuteExW(&mut info) } {
        if error.code() == ERROR_CANCELLED.to_hresult() {
            return Ok(None);
        }
        return Err(WinError::api("ShellExecuteExW", error));
    }
    let process = info.hProcess;
    if process.is_invalid() {
        return Err(WinError::Unsupported("elevated process handle"));
    }
    let millis = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX);
    // SAFETY: `process` es el proceso propio que acabamos de abrir; se cierra una sola vez.
    let result = unsafe {
        let waited = WaitForSingleObject(process, millis);
        let mut code = 1u32;
        let read = GetExitCodeProcess(process, &mut code);
        let _ = CloseHandle(process);
        if waited != WAIT_OBJECT_0 {
            return Err(WinError::Timeout("elevated process"));
        }
        read.map(|_| code)
    };
    result
        .map(Some)
        .map_err(|error| WinError::api("GetExitCodeProcess", error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn this_process_is_in_the_snapshot() {
        let names = process_names();
        let own = names.get(&std::process::id()).expect("el propio proceso");
        assert!(own.to_lowercase().ends_with(".exe"));
        let me = ProcessInfo {
            pid: std::process::id(),
            exe: own.to_uppercase(),
        };
        assert!(is_running(&me), "sin distinguir mayúsculas");
        let other = ProcessInfo {
            pid: std::process::id(),
            exe: "otro.exe".into(),
        };
        assert!(!is_running(&other), "mismo pid, otro ejecutable");
    }
}
