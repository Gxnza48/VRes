//! Controles por hardware del monitor (DDC/CI, para `vres-cli ddc`) y el
//! anonimizador del reporte de diagnóstico.

use serde::{Deserialize, Serialize};

/// Un control por hardware del monitor (código VCP del estándar MCCS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VcpValue {
    pub current: u32,
    pub max: u32,
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
