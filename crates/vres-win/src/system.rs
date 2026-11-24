//! Datos del sistema para el reporte de diagnóstico.

use windows::Win32::System::Registry::{
    HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegGetValueW,
};
use windows::core::{HSTRING, PCWSTR};

const CURRENT_VERSION: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";

fn read_string(name: &str) -> Option<String> {
    let (key, value) = (HSTRING::from(CURRENT_VERSION), HSTRING::from(name));
    let mut buffer = [0u16; 128];
    let mut size = (buffer.len() * 2) as u32;
    // SAFETY: `buffer` y `size` viven durante la llamada y `size` es su tamaño en bytes.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(key.as_ptr()),
            PCWSTR(value.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if status.is_err() {
        return None;
    }
    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..len]))
}

fn read_dword(name: &str) -> Option<u32> {
    let (key, value) = (HSTRING::from(CURRENT_VERSION), HSTRING::from(name));
    let mut data = 0u32;
    let mut size = 4u32;
    // SAFETY: `data` y `size` viven durante la llamada.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(key.as_ptr()),
            PCWSTR(value.as_ptr()),
            RRF_RT_REG_DWORD,
            None,
            Some((&raw mut data).cast()),
            Some(&mut size),
        )
    };
    status.is_ok().then_some(data)
}

/// "Windows 11 Home 24H2 (26100.1234)", leído del registro.
pub fn windows_version() -> Option<String> {
    let product = read_string("ProductName").unwrap_or_else(|| "Windows".into());
    let build: u32 = read_string("CurrentBuild")?.parse().ok()?;
    // El registro sigue diciendo "Windows 10" en Windows 11.
    let product = if build >= 22000 {
        product.replace("Windows 10", "Windows 11")
    } else {
        product
    };
    let display = read_string("DisplayVersion").unwrap_or_default();
    let ubr = read_dword("UBR").unwrap_or(0);
    Some(format!("{product} {display} ({build}.{ubr})").replace("  ", " "))
}
