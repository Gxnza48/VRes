//! Vibrance digital del driver: backends y conversión entre la escala de VRes
//! (0–100, 50 = el valor normal del driver) y el rango real de cada driver.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Driver que maneja la vibrance de una pantalla.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum VibranceBackend {
    /// NVIDIA (NVAPI, "Digital Vibrance").
    Nvidia,
    /// AMD (ADL, saturación de la pantalla).
    Amd,
}

/// Rango y valor actual del driver para una pantalla.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VibranceInfo {
    pub backend: VibranceBackend,
    pub current: i32,
    pub min: i32,
    pub max: i32,
    /// El valor "normal" del driver (sin realce).
    pub default: i32,
}

impl VibranceInfo {
    /// Nivel del driver para un valor de VRes: 0 → mínimo, 50 → normal,
    /// 100 → máximo (por tramos, porque el normal no siempre está al medio).
    pub fn raw_for(&self, value: f32) -> i32 {
        let value = value.clamp(0.0, 100.0);
        let (min, max, default) = (self.min as f32, self.max as f32, self.default as f32);
        let raw = if value >= 50.0 {
            default + (max - default) * (value - 50.0) / 50.0
        } else {
            min + (default - min) * value / 50.0
        };
        (raw.round() as i32).clamp(self.min, self.max)
    }

    /// Valor de VRes para un nivel del driver (la inversa de [`raw_for`]).
    ///
    /// [`raw_for`]: VibranceInfo::raw_for
    pub fn value_for(&self, raw: i32) -> f32 {
        let raw = raw.clamp(self.min, self.max) as f32;
        let (min, max, default) = (self.min as f32, self.max as f32, self.default as f32);
        if raw >= default {
            if max > default {
                50.0 + 50.0 * (raw - default) / (max - default)
            } else {
                50.0
            }
        } else if default > min {
            50.0 * (raw - min) / (default - min)
        } else {
            50.0
        }
    }
}

/// Vibrance que aproxima una saturación cuando la matriz de color no se puede
/// usar (hay monitores sin tildar): cada 4 puntos de saturación sobre 100,
/// un punto de vibrance.
pub fn approximate_with_saturation(vibrance: f32, saturation: f32) -> f32 {
    (vibrance + (saturation - 100.0) / 4.0).clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(min: i32, max: i32, default: i32) -> VibranceInfo {
        VibranceInfo {
            backend: VibranceBackend::Amd,
            current: default,
            min,
            max,
            default,
        }
    }

    #[test]
    fn fifty_is_the_driver_default() {
        assert_eq!(info(0, 100, 50).raw_for(50.0), 50, "NVIDIA (Ex)");
        assert_eq!(info(0, 200, 100).raw_for(50.0), 100, "AMD");
        assert_eq!(
            info(0, 63, 0).raw_for(50.0),
            0,
            "NVIDIA clásico: 0 es normal"
        );
    }

    #[test]
    fn the_ends_reach_the_driver_limits() {
        let amd = info(0, 200, 100);
        assert_eq!(amd.raw_for(0.0), 0);
        assert_eq!(amd.raw_for(100.0), 200);
        assert_eq!(amd.raw_for(75.0), 150);
        let classic = info(0, 63, 0);
        assert_eq!(classic.raw_for(100.0), 63);
        assert_eq!(classic.raw_for(10.0), 0, "no puede bajar del normal");
        assert_eq!(amd.raw_for(250.0), 200, "se recorta");
    }

    #[test]
    fn value_for_inverts_raw_for() {
        for driver in [info(0, 100, 50), info(0, 200, 100)] {
            for value in [0.0, 12.0, 50.0, 63.0, 100.0] {
                let back = driver.value_for(driver.raw_for(value));
                assert!((back - value).abs() <= 1.0, "{value} → {back}");
            }
        }
    }

    #[test]
    fn saturation_approximation_is_clamped() {
        assert_eq!(approximate_with_saturation(50.0, 100.0), 50.0);
        assert_eq!(approximate_with_saturation(50.0, 200.0), 75.0);
        assert_eq!(approximate_with_saturation(50.0, 300.0), 100.0);
        assert_eq!(approximate_with_saturation(80.0, 300.0), 100.0);
        assert_eq!(approximate_with_saturation(50.0, 0.0), 25.0);
    }
}
