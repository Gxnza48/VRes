//! Ventana del crosshair: transparente por píxel, siempre encima, sin clics ni
//! foco, y fuera de Alt+Tab y de la barra de tareas.
//!
//! Se pinta con `UpdateLayeredWindow` (BGRA premultiplicado): Windows la
//! compone solo, sin repintar mientras el dibujo no cambie.

use std::ffi::c_void;
use std::ptr::null_mut;

use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION,
    CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, ReleaseDC,
    SelectObject,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, HWND_TOPMOST, RegisterClassExW, SW_HIDE,
    SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowDisplayAffinity,
    SetWindowPos, ShowWindow, ULW_ALPHA, UpdateLayeredWindow, WDA_EXCLUDEFROMCAPTURE, WDA_NONE,
    WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use crate::{WinError, WinResult};

const CLASS: PCWSTR = w!("VRes.Crosshair");

unsafe extern "system" fn overlay_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: procesamiento por defecto de Windows para nuestra ventana.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// La ventana del crosshair. Vive en el hilo que la creó (no es `Send`).
#[derive(Debug)]
pub struct CrosshairOverlay {
    hwnd: HWND,
    visible: bool,
}

impl CrosshairOverlay {
    /// Crea la ventana (todavía oculta). El hilo tiene que procesar mensajes.
    pub fn create() -> WinResult<Self> {
        // SAFETY: devuelve el módulo del ejecutable actual.
        let module = unsafe { GetModuleHandleW(PCWSTR::null()) }
            .map_err(|e| WinError::api("GetModuleHandleW", e))?;
        let instance = HINSTANCE(module.0);
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(overlay_proc),
            hInstance: instance,
            lpszClassName: CLASS,
            ..Default::default()
        };
        // SAFETY: estructura válida; si la clase ya existe, se reutiliza.
        unsafe { RegisterClassExW(&class) };
        // SAFETY: la clase está registrada; ventana sin parámetros de creación.
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TRANSPARENT
                    | WS_EX_TOPMOST
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOACTIVATE,
                CLASS,
                w!("VRes Crosshair"),
                WS_POPUP,
                0,
                0,
                1,
                1,
                None,
                None,
                Some(instance),
                None,
            )
        }
        .map_err(|e| WinError::api("CreateWindowExW", e))?;
        Ok(Self {
            hwnd,
            visible: false,
        })
    }

    /// Muestra `bgra` (premultiplicado, `size × size`) con su esquina superior
    /// izquierda en `(x, y)` del escritorio virtual.
    pub fn show(&mut self, bgra: &[u8], size: u32, x: i32, y: i32) -> WinResult<()> {
        let bytes = (size as usize) * (size as usize) * 4;
        if size == 0 || bgra.len() != bytes {
            return Err(WinError::Unsupported("crosshair image size"));
        }
        // SAFETY: DC de la pantalla; se libera abajo.
        let screen = unsafe { GetDC(None) };
        // SAFETY: DC de memoria compatible; se libera abajo.
        let memory = unsafe { CreateCompatibleDC(Some(screen)) };
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size as i32,
                // Negativo: filas de arriba hacia abajo, como el dibujo.
                biHeight: -(size as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = null_mut();
        // SAFETY: `info` describe un DIB de 32 bits; Windows devuelve en `bits`
        // un búfer de exactamente `bytes` bytes.
        let bitmap =
            unsafe { CreateDIBSection(Some(memory), &info, DIB_RGB_COLORS, &mut bits, None, 0) };
        let result = match bitmap {
            Ok(bitmap) if !bits.is_null() => {
                // SAFETY: `bits` apunta a `bytes` bytes escribibles del DIB.
                unsafe { std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits.cast::<u8>(), bytes) };
                // SAFETY: el bitmap y el DC son nuestros.
                let previous = unsafe { SelectObject(memory, bitmap.into()) };
                let blend = BLENDFUNCTION {
                    BlendOp: AC_SRC_OVER as u8,
                    BlendFlags: 0,
                    SourceConstantAlpha: 255,
                    AlphaFormat: AC_SRC_ALPHA as u8,
                };
                let position = POINT { x, y };
                let extent = SIZE {
                    cx: size as i32,
                    cy: size as i32,
                };
                let source = POINT { x: 0, y: 0 };
                // SAFETY: todos los punteros apuntan a variables locales vivas.
                let updated = unsafe {
                    UpdateLayeredWindow(
                        self.hwnd,
                        Some(screen),
                        Some(&position),
                        Some(&extent),
                        Some(memory),
                        Some(&source),
                        COLORREF(0),
                        Some(&blend),
                        ULW_ALPHA,
                    )
                };
                // SAFETY: se devuelve el objeto anterior y se libera el bitmap.
                unsafe {
                    SelectObject(memory, previous);
                    let _ = DeleteObject(bitmap.into());
                }
                updated.map_err(|e| WinError::api("UpdateLayeredWindow", e))
            }
            Ok(bitmap) => {
                // SAFETY: bitmap nuestro, sin seleccionar.
                let _ = unsafe { DeleteObject(bitmap.into()) };
                Err(WinError::Unsupported("CreateDIBSection returned no bits"))
            }
            Err(error) => Err(WinError::api("CreateDIBSection", error)),
        };
        // SAFETY: liberan los DC que obtuvimos arriba.
        unsafe {
            let _ = DeleteDC(memory);
            ReleaseDC(None, screen);
        }
        result?;
        if !self.visible {
            // SAFETY: ventana nuestra; se muestra sin activarla.
            let _ = unsafe { ShowWindow(self.hwnd, SW_SHOWNOACTIVATE) };
            self.visible = true;
        }
        self.raise();
        Ok(())
    }

    /// La oculta sin destruirla.
    pub fn hide(&mut self) {
        if self.visible {
            // SAFETY: ventana nuestra.
            let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
            self.visible = false;
        }
    }

    /// Vuelve a ponerla encima de todo (tras un cambio de ventana en primer plano).
    pub fn raise(&self) {
        // SAFETY: ventana nuestra; no se mueve, no cambia de tamaño ni se activa.
        let _ = unsafe {
            SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )
        };
    }

    /// Que no aparezca en capturas ni transmisiones (Windows 10 2004+).
    /// Devuelve `false` si Windows no lo admite.
    pub fn set_hidden_from_capture(&self, hidden: bool) -> bool {
        let affinity = if hidden {
            WDA_EXCLUDEFROMCAPTURE
        } else {
            WDA_NONE
        };
        // SAFETY: ventana nuestra, de nivel superior.
        unsafe { SetWindowDisplayAffinity(self.hwnd, affinity) }.is_ok()
    }
}

impl Drop for CrosshairOverlay {
    fn drop(&mut self) {
        // SAFETY: destruye la ventana que creamos, una sola vez.
        let _ = unsafe { DestroyWindow(self.hwnd) };
    }
}
