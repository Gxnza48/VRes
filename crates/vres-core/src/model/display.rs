//! Información de cada pantalla, tal como la ve VRes.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Fabricante de la placa de video.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Other,
}

impl GpuVendor {
    /// A partir del identificador PCI del fabricante.
    pub fn from_pci_vendor_id(id: u32) -> Self {
        match id {
            0x10DE => Self::Nvidia,
            0x1002 | 0x1022 => Self::Amd,
            0x8086 => Self::Intel,
            _ => Self::Other,
        }
    }
}

/// Placa de video que maneja una pantalla.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AdapterInfo {
    pub vendor: GpuVendor,
    /// Nombre comercial (por ejemplo "NVIDIA GeForce RTX 5060 Laptop GPU").
    pub name: String,
}

/// Rectángulo en píxeles físicos del escritorio virtual.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RectPx {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl RectPx {
    /// Centro del rectángulo.
    pub fn center(&self) -> (i32, i32) {
        (
            self.x + (self.width / 2) as i32,
            self.y + (self.height / 2) as i32,
        )
    }
}

/// Estado de HDR de una pantalla.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HdrState {
    /// La pantalla admite HDR.
    pub supported: bool,
    /// HDR está activo ahora.
    pub enabled: bool,
}

/// Una pantalla activa.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DisplayInfo {
    /// Identificador estable entre reinicios (no cambia si Windows renumera).
    pub id: String,
    /// Nombre GDI, por ejemplo `\\.\DISPLAY1` (puede cambiar).
    pub gdi_name: String,
    /// Nombre del monitor, por ejemplo "MAG 255F E20".
    pub friendly_name: String,
    /// Placa de video que lo maneja.
    pub adapter: AdapterInfo,
    /// Posición y tamaño en el escritorio virtual.
    pub bounds: RectPx,
    /// Es el monitor principal.
    pub primary: bool,
    /// Es el panel interno de una notebook.
    pub internal: bool,
    /// Frecuencia de actualización exacta, en Hz.
    pub refresh_hz: f64,
    pub hdr: HdrState,
}

/// Identificador estable de una pantalla a partir de su ruta de dispositivo
/// (`monitorDevicePath`): `d-` + 16 dígitos hexadecimales (FNV-1a de 64 bits).
pub fn stable_display_id(device_path: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in device_path.to_ascii_lowercase().bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("d-{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_id_is_deterministic_and_case_insensitive() {
        let a = stable_display_id(r"\\?\DISPLAY#MSI5BC2#5&2d5dc5b1&0&UID4352#{e6f07b5f}");
        let b = stable_display_id(r"\\?\display#msi5bc2#5&2D5DC5B1&0&uid4352#{E6F07B5F}");
        assert_eq!(a, b);
        assert!(a.starts_with("d-"));
        assert_eq!(a.len(), 18);
        assert_ne!(a, stable_display_id("otro"));
    }

    #[test]
    fn vendor_from_pci_id() {
        assert_eq!(GpuVendor::from_pci_vendor_id(0x10DE), GpuVendor::Nvidia);
        assert_eq!(GpuVendor::from_pci_vendor_id(0x1002), GpuVendor::Amd);
        assert_eq!(GpuVendor::from_pci_vendor_id(0x8086), GpuVendor::Intel);
        assert_eq!(GpuVendor::from_pci_vendor_id(0x1414), GpuVendor::Other);
    }
}
