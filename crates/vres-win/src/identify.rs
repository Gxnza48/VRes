//! "Identificar pantallas": un número grande en el centro de cada monitor.

use std::ffi::c_void;

use vres_core::model::display::RectPx;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontW, CreateSolidBrush, DT_CENTER, DT_SINGLELINE, DT_VCENTER, DeleteObject,
    DrawTextW, EndPaint, FW_SEMIBOLD, FillRect, PAINTSTRUCT, SelectObject, SetBkMode, SetTextColor,
    TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CreateWindowExW, DefWindowProcW, DestroyWindow, GWLP_USERDATA, GetClientRect,
    GetWindowLongPtrW, RegisterClassExW, SW_SHOWNOACTIVATE, SetWindowLongPtrW, ShowWindow,
    WM_NCCREATE, WM_PAINT, WNDCLASSEXW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use crate::{WinError, WinResult};

const CLASS: PCWSTR = w!("VRes.Identify");
const SIZE: i32 = 240;
const BACKGROUND: COLORREF = COLORREF(0x0000_0000); // negro
const FOREGROUND: COLORREF = COLORREF(0x0012_FA9E); // verde lima #9EFA12 (COLORREF es BGR)

/// Las ventanas con los números; se cierran al destruirse.
#[derive(Debug, Default)]
pub struct IdentifyOverlay {
    windows: Vec<HWND>,
}

impl IdentifyOverlay {
    /// Muestra `label` en el centro de cada rectángulo. Debe usarse en un hilo
    /// que procese mensajes.
    pub fn show(entries: &[(RectPx, u32)]) -> WinResult<Self> {
        // SAFETY: devuelve el módulo del ejecutable actual.
        let module = unsafe { GetModuleHandleW(PCWSTR::null()) }
            .map_err(|e| WinError::api("GetModuleHandleW", e))?;
        let instance = HINSTANCE(module.0);
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(identify_proc),
            hInstance: instance,
            lpszClassName: CLASS,
            ..Default::default()
        };
        // SAFETY: estructura válida; si la clase ya existe, se reutiliza.
        unsafe { RegisterClassExW(&class) };

        let mut overlay = Self::default();
        for (bounds, number) in entries {
            let (x, y) = bounds.center();
            // SAFETY: la clase está registrada; el número viaja como parámetro
            // de creación (un entero, no un puntero).
            let hwnd = unsafe {
                CreateWindowExW(
                    WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TRANSPARENT,
                    CLASS,
                    w!("VRes"),
                    WS_POPUP,
                    x - SIZE / 2,
                    y - SIZE / 2,
                    SIZE,
                    SIZE,
                    None,
                    None,
                    Some(instance),
                    Some(*number as usize as *const c_void),
                )
            }
            .map_err(|e| WinError::api("CreateWindowExW", e))?;
            // SAFETY: hwnd es la ventana que acabamos de crear en este hilo.
            unsafe {
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }
            overlay.windows.push(hwnd);
        }
        Ok(overlay)
    }
}

impl Drop for IdentifyOverlay {
    fn drop(&mut self) {
        for hwnd in self.windows.drain(..) {
            // SAFETY: destruye cada ventana que creamos, una sola vez.
            let _ = unsafe { DestroyWindow(hwnd) };
        }
    }
}

extern "system" fn identify_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        // SAFETY: en WM_NCCREATE, lparam apunta a un CREATESTRUCTW válido; el
        // parámetro de creación es el número a mostrar.
        unsafe {
            let create = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        }
    }
    if msg == WM_PAINT {
        let mut paint = PAINTSTRUCT::default();
        // SAFETY: secuencia estándar de pintado sobre nuestra propia ventana;
        // pincel y fuente se crean y se liberan dentro del bloque.
        unsafe {
            let number = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            let mut text: Vec<u16> = number.to_string().encode_utf16().collect();
            let hdc = BeginPaint(hwnd, &mut paint);
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);
            let brush = CreateSolidBrush(BACKGROUND);
            FillRect(hdc, &rect, brush);
            let font = CreateFontW(
                180,
                0,
                0,
                0,
                FW_SEMIBOLD.0 as i32,
                0,
                0,
                0,
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                0,
                w!("Segoe UI"),
            );
            let previous = SelectObject(hdc, font.into());
            SetBkMode(hdc, TRANSPARENT);
            SetTextColor(hdc, FOREGROUND);
            DrawTextW(
                hdc,
                &mut text,
                &mut rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );
            SelectObject(hdc, previous);
            let _ = DeleteObject(font.into());
            let _ = DeleteObject(brush.into());
            let _ = EndPaint(hwnd, &paint);
        }
        return LRESULT(0);
    }
    // SAFETY: procesamiento por defecto con los mismos parámetros.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}
