//! Pestaña Monitor: controles por hardware (DDC/CI) de los monitores externos
//! y brillo del panel de las notebooks. A diferencia del color, son ajustes
//! del propio monitor (como su menú): VRes no los deshace al salir, pero
//! guarda los valores de antes para volver a ellos con un botón.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Un control por hardware del monitor (código VCP del estándar MCCS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DdcControl {
    Brightness,
    Contrast,
    RedGain,
    GreenGain,
    BlueGain,
}

impl DdcControl {
    /// Todos, en el orden en que se muestran.
    pub const ALL: [Self; 5] = [
        Self::Brightness,
        Self::Contrast,
        Self::RedGain,
        Self::GreenGain,
        Self::BlueGain,
    ];

    /// Código VCP (MCCS).
    pub fn vcp_code(self) -> u8 {
        match self {
            Self::Brightness => 0x10,
            Self::Contrast => 0x12,
            Self::RedGain => 0x16,
            Self::GreenGain => 0x18,
            Self::BlueGain => 0x1A,
        }
    }
}

/// Valor de un control: el actual y el máximo (el mínimo es 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VcpValue {
    pub current: u32,
    pub max: u32,
}

/// Lo que se puede ajustar por hardware en una pantalla.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MonitorDetails {
    pub display_id: String,
    /// Escala de Windows (%), si se pudo leer.
    pub scale_percent: Option<u32>,
    /// Controles DDC/CI que responde el monitor (vacío: no lo admite).
    pub ddc: BTreeMap<DdcControl, VcpValue>,
    /// Brillo del panel de una notebook (0–100), por WMI.
    pub panel_brightness: Option<u32>,
    /// Hay valores de antes de VRes para volver a ellos.
    pub has_initial: bool,
}

/// Valores de hardware de una pantalla antes de que VRes los cambiara.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InitialValues {
    pub ddc: BTreeMap<DdcControl, u32>,
    pub panel_brightness: Option<u32>,
}

/// `state/hardware-initial.json`: valores de antes de VRes, por pantalla.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HardwareInitial {
    pub schema_version: u32,
    pub displays: BTreeMap<String, InitialValues>,
}

impl HardwareInitial {
    /// Anota el valor de un control la primera vez que VRes lo cambia.
    /// Devuelve `true` si lo anotó (hay que guardar el archivo).
    pub fn remember_ddc(&mut self, display: &str, control: DdcControl, current: u32) -> bool {
        let values = self.displays.entry(display.to_owned()).or_default();
        if values.ddc.contains_key(&control) {
            return false;
        }
        values.ddc.insert(control, current);
        true
    }

    /// Anota el brillo del panel la primera vez que VRes lo cambia.
    pub fn remember_panel(&mut self, display: &str, current: u32) -> bool {
        let values = self.displays.entry(display.to_owned()).or_default();
        if values.panel_brightness.is_some() {
            return false;
        }
        values.panel_brightness = Some(current);
        true
    }

    /// `true` si hay algo para volver atrás en esa pantalla.
    pub fn has(&self, display: &str) -> bool {
        self.displays
            .get(display)
            .is_some_and(|values| !values.ddc.is_empty() || values.panel_brightness.is_some())
    }

    /// Saca los valores de una pantalla (al restaurarlos).
    pub fn take(&mut self, display: &str) -> Option<InitialValues> {
        self.displays.remove(display)
    }
}

/// Reemplaza datos personales (nombre de usuario, carpeta de usuario, nombre
/// del equipo) en un texto de diagnóstico. Distingue sin mayúsculas: Windows
/// no las distingue en las rutas.
pub fn anonymize(text: &str, secrets: &[(&str, &str)]) -> String {
    let mut out = text.to_owned();
    // Primero los más largos: la carpeta de usuario contiene el nombre.
    let mut ordered: Vec<&(&str, &str)> = secrets
        .iter()
        .filter(|(secret, _)| secret.chars().count() >= 2)
        .collect();
    ordered.sort_by_key(|(secret, _)| std::cmp::Reverse(secret.len()));
    for (secret, placeholder) in ordered {
        out = replace_ignore_case(&out, secret, placeholder);
    }
    out
}

fn replace_ignore_case(text: &str, needle: &str, replacement: &str) -> String {
    let lower_text = text.to_lowercase();
    let lower_needle = needle.to_lowercase();
    // Si pasar a minúsculas cambia las longitudes (algunos caracteres raros),
    // se reemplaza solo la coincidencia exacta.
    if lower_text.len() != text.len() || lower_needle.len() != needle.len() {
        return text.replace(needle, replacement);
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = 0;
    let mut from = 0;
    while let Some(found) = lower_text[from..].find(&lower_needle) {
        let start = from + found;
        out.push_str(&text[rest..start]);
        out.push_str(replacement);
        rest = start + needle.len();
        from = rest;
    }
    out.push_str(&text[rest..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vcp_codes_follow_mccs() {
        assert_eq!(DdcControl::Brightness.vcp_code(), 0x10);
        assert_eq!(DdcControl::Contrast.vcp_code(), 0x12);
        assert_eq!(DdcControl::BlueGain.vcp_code(), 0x1A);
    }

    #[test]
    fn the_first_value_is_kept() {
        let mut initial = HardwareInitial::default();
        assert!(!initial.has("d-1"));
        assert!(initial.remember_ddc("d-1", DdcControl::Brightness, 70));
        assert!(
            !initial.remember_ddc("d-1", DdcControl::Brightness, 40),
            "el de antes de VRes no se pisa"
        );
        assert!(initial.remember_panel("d-2", 55));
        assert!(!initial.remember_panel("d-2", 80));
        assert!(initial.has("d-1") && initial.has("d-2"));
        let values = initial.take("d-1").expect("valores");
        assert_eq!(values.ddc[&DdcControl::Brightness], 70);
        assert!(!initial.has("d-1"));
    }

    #[test]
    fn initial_values_round_trip_as_json() {
        let mut initial = HardwareInitial {
            schema_version: 1,
            ..HardwareInitial::default()
        };
        initial.remember_ddc("d-1", DdcControl::RedGain, 50);
        let json = serde_json::to_string(&initial).expect("json");
        assert!(json.contains("\"redGain\":50"), "{json}");
        let back: HardwareInitial = serde_json::from_str(&json).expect("válido");
        assert_eq!(back, initial);
    }

    #[test]
    fn anonymize_removes_the_user_and_the_computer() {
        let text = r"C:\Users\Maria\AppData\Local\VRes\logs at MARIA-PC by maria";
        let out = anonymize(
            text,
            &[
                (r"C:\Users\maria", r"C:\Users\<usuario>"),
                ("maria", "<usuario>"),
                ("MARIA-PC", "<equipo>"),
            ],
        );
        assert_eq!(
            out,
            r"C:\Users\<usuario>\AppData\Local\VRes\logs at <equipo> by <usuario>"
        );
        assert!(!out.to_lowercase().contains("maria"));
        assert_eq!(
            anonymize("abc", &[("", "x"), ("a", "x")]),
            "abc",
            "no reemplaza secretos de menos de 2 letras"
        );
    }
}
