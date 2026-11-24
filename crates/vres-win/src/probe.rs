//! Sondas para verificar en pantalla lo que ninguna API confirma.
//!
//! - [`ColorPatch`]: un cuadradito de color conocido, siempre encima, que no
//!   roba el foco ni los clics.
//! - [`read_screen_pixel`]: lee un píxel de la imagen compuesta del escritorio.
//!   La captura GDI incluye la matriz de color de Windows (pero no la rampa de
//!   gamma, que se aplica después, al enviar la imagen al monitor).

use std::ffi::c_void;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CAPTUREBLT, CreateCompatibleBitmap, CreateCompatibleDC, CreateSolidBrush,
    DeleteDC, DeleteObject, EndPaint, FillRect, GetDC, GetPixel, PAINTSTRUCT, ReleaseDC, SRCCOPY,
    SelectObject,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GWLP_USERDATA,
    GetClientRect, GetWindowLongPtrW, MSG, PM_REMOVE, PeekMessageW, RegisterClassExW,
    SW_SHOWNOACTIVATE, SetWindowLongPtrW, ShowWindow, TranslateMessage, WM_NCCREATE, WM_PAINT,
    WNDCLASSEXW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use crate::{WinError, WinResult};

const PATCH_CLASS: PCWSTR = w!("VRes.Probe");

fn colorref(rgb: [u8; 3]) -> COLORREF {
    COLORREF(u32::from(rgb[0]) | (u32::from(rgb[1]) << 8) | (u32::from(rgb[2]) << 16))
}

/// Procesa los mensajes pendientes del hilo durante `duration` (para que las
/// ventanas se pinten y el compositor muestre el resultado).
pub fn pump_for(duration: Duration) {
    let start = Instant::now();
    let mut message = MSG::default();
    while start.elapsed() < duration {
        // SAFETY: `message` vive durante todo el bucle.
        while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
            // SAFETY: `message` fue completado por PeekMessageW.
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Cuadrado de color sólido, siempre encima, transparente a los clics.
#[derive(Debug)]
pub struct ColorPatch {
    hwnd: HWND,
}

impl ColorPatch {
    /// Muestra un cuadrado de `size` píxeles centrado en `(x, y)`.
    ///
    /// Debe usarse en un hilo que procese mensajes (ver [`pump_for`]).
    pub fn show(x: i32, y: i32, size: i32, rgb: [u8; 3]) -> WinResult<Self> {
        // SAFETY: devuelve el módulo del ejecutable actual.
        let module = unsafe { GetModuleHandleW(PCWSTR::null()) }
            .map_err(|e| WinError::api("GetModuleHandleW", e))?;
        let instance = HINSTANCE(module.0);
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(patch_proc),
            hInstance: instance,
            lpszClassName: PATCH_CLASS,
            ..Default::default()
        };
        // SAFETY: estructura válida; si la clase ya existe, se reutiliza.
        unsafe { RegisterClassExW(&class) };

        let color = colorref(rgb).0 as usize;
        // SAFETY: la clase está registrada; `color` viaja como parámetro de
        // creación (un entero, no un puntero).
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT,
                PATCH_CLASS,
                w!("VRes probe"),
                WS_POPUP,
                x - size / 2,
                y - size / 2,
                size,
                size,
                None,
                None,
                Some(instance),
                Some(color as *const c_void),
            )
        }
        .map_err(|e| WinError::api("CreateWindowExW", e))?;

        // SAFETY: hwnd es la ventana que acabamos de crear en este hilo.
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        Ok(Self { hwnd })
    }
}

impl Drop for ColorPatch {
    fn drop(&mut self) {
        // SAFETY: destruye la ventana que creamos, una sola vez.
        let _ = unsafe { DestroyWindow(self.hwnd) };
        pump_for(Duration::from_millis(30));
    }
}

extern "system" fn patch_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        // SAFETY: en WM_NCCREATE, lparam apunta a un CREATESTRUCTW válido; el
        // parámetro de creación es el color (un entero).
        unsafe {
            let create = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        }
    }
    if msg == WM_PAINT {
        let mut paint = PAINTSTRUCT::default();
        // SAFETY: secuencia estándar de pintado sobre nuestra propia ventana;
        // el pincel se crea y se libera dentro del bloque.
        unsafe {
            let color = COLORREF(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as u32);
            let hdc = BeginPaint(hwnd, &mut paint);
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);
            let brush = CreateSolidBrush(color);
            FillRect(hdc, &rect, brush);
            let _ = DeleteObject(brush.into());
            let _ = EndPaint(hwnd, &paint);
        }
        return LRESULT(0);
    }
    // SAFETY: procesamiento por defecto con los mismos parámetros.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// Lee el color de un píxel de la imagen compuesta del escritorio
/// (coordenadas físicas del escritorio virtual).
pub fn read_screen_pixel(x: i32, y: i32) -> WinResult<[u8; 3]> {
    // SAFETY: se obtiene el DC de la pantalla y se libera al final; la copia
    // va a un mapa de bits de 1×1 propio, que también se libera.
    unsafe {
        let screen = GetDC(None);
        if screen.is_invalid() {
            return Err(WinError::last("GetDC"));
        }
        let memory = CreateCompatibleDC(Some(screen));
        let bitmap = CreateCompatibleBitmap(screen, 1, 1);
        let previous = SelectObject(memory, bitmap.into());
        let copied = BitBlt(memory, 0, 0, 1, 1, Some(screen), x, y, SRCCOPY | CAPTUREBLT);
        let color = GetPixel(memory, 0, 0);
        SelectObject(memory, previous);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(memory);
        ReleaseDC(None, screen);
        copied.map_err(|e| WinError::api("BitBlt", e))?;
        Ok([
            (color.0 & 0xFF) as u8,
            ((color.0 >> 8) & 0xFF) as u8,
            ((color.0 >> 16) & 0xFF) as u8,
        ])
    }
}
