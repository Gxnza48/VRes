//! Rangos de cada parámetro: una sola fuente de verdad para el motor y la
//! interfaz. Se exportan a `src/lib/bindings/ranges.json` con `cargo test`, y
//! los deslizadores de la interfaz los leen de ahí.

use serde::Serialize;

/// Rango de un parámetro numérico.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Range {
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub step: f32,
}

impl Range {
    const fn new(min: f32, max: f32, default: f32, step: f32) -> Self {
        Self {
            min,
            max,
            default,
            step,
        }
    }

    /// Recorta un valor al rango; los valores no finitos vuelven al defecto.
    pub fn clamp(&self, value: f32) -> f32 {
        if value.is_finite() {
            value.clamp(self.min, self.max)
        } else {
            self.default
        }
    }
}

/// Saturación (%): 100 es la imagen tal como viene.
pub const SATURATION: Range = Range::new(0.0, 300.0, 100.0, 1.0);
/// Vibrance digital estilo panel de NVIDIA: 50 es neutro.
pub const VIBRANCE: Range = Range::new(0.0, 100.0, 50.0, 1.0);
/// Brillo (%).
pub const BRIGHTNESS: Range = Range::new(50.0, 170.0, 100.0, 1.0);
/// Contraste (%).
pub const CONTRAST: Range = Range::new(50.0, 150.0, 100.0, 1.0);
/// Gamma: mayor que 1 aclara.
pub const GAMMA: Range = Range::new(0.5, 1.5, 1.0, 0.01);
/// Temperatura: negativo enfría, positivo calienta.
pub const TEMPERATURE: Range = Range::new(-100.0, 100.0, 0.0, 1.0);
/// Regiones tonales (altas luces, sombras, negros, blancos) y tintes.
pub const TONAL: Range = Range::new(-100.0, 100.0, 0.0, 1.0);
/// Ganancia por canal (1 = sin cambio).
pub const GAIN: Range = Range::new(0.5, 1.5, 1.0, 0.01);
/// Giro de tono, en grados.
pub const HUE_SHIFT: Range = Range::new(-180.0, 180.0, 0.0, 1.0);
/// Porcentajes genéricos (cantidad, intensidad, calidez).
pub const PERCENT: Range = Range::new(0.0, 100.0, 0.0, 1.0);
/// Atenuación del cuidado visual (%).
pub const DIM: Range = Range::new(0.0, 80.0, 0.0, 1.0);
/// Tono a oscurecer, en grados.
pub const HUE: Range = Range::new(0.0, 360.0, 0.0, 1.0);
/// Gamma propia de cada monitor (multiplica la del look).
pub const TRIM_GAMMA: Range = Range::new(0.7, 1.3, 1.0, 0.01);

/// Todos los rangos, con el nombre con el que los usa la interfaz.
pub const ALL: &[(&str, Range)] = &[
    ("saturation", SATURATION),
    ("vibrance", VIBRANCE),
    ("brightness", BRIGHTNESS),
    ("contrast", CONTRAST),
    ("gamma", GAMMA),
    ("temperature", TEMPERATURE),
    ("tonal", TONAL),
    ("gain", GAIN),
    ("hueShift", HUE_SHIFT),
    ("percent", PERCENT),
    ("dim", DIM),
    ("hue", HUE),
    ("trimGamma", TRIM_GAMMA),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_inside_their_ranges() {
        for (name, range) in ALL {
            assert!(range.min < range.max, "{name}");
            assert!(
                (range.min..=range.max).contains(&range.default),
                "{name}: el valor por defecto está fuera del rango"
            );
            assert!(range.step > 0.0, "{name}");
        }
    }

    #[test]
    fn clamp_handles_invalid_values() {
        assert_eq!(SATURATION.clamp(f32::NAN), 100.0);
        assert_eq!(SATURATION.clamp(f32::INFINITY), 100.0);
        assert_eq!(SATURATION.clamp(999.0), 300.0);
        assert_eq!(SATURATION.clamp(-5.0), 0.0);
    }

    /// Exporta los rangos para la interfaz (`src/lib/bindings/ranges.json`).
    #[test]
    fn export_ranges_json() {
        let Some(dir) = std::env::var_os("TS_RS_EXPORT_DIR") else {
            return;
        };
        let map: serde_json::Map<String, serde_json::Value> = ALL
            .iter()
            .map(|(name, range)| {
                let value = serde_json::to_value(range).expect("serializable");
                ((*name).to_owned(), value)
            })
            .collect();
        let json = serde_json::to_string_pretty(&map).expect("serializable") + "\n";
        let path = std::path::Path::new(&dir).join("ranges.json");
        std::fs::create_dir_all(std::path::Path::new(&dir)).expect("carpeta de bindings");
        std::fs::write(path, json).expect("ranges.json");
    }
}
