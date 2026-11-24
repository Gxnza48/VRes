//! El *look*: todo lo que define cómo se ven los colores.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::ranges;

/// Un look completo. Los valores están en las unidades de la interfaz
/// (porcentajes, grados, −100…100); el motor los convierte.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct Look {
    /// 0–300 %, 100 = sin cambio. Capa de matriz (todo el escritorio).
    pub saturation: f32,
    /// 0–100, 50 = neutro. Vibrance del driver (por monitor).
    pub vibrance: f32,
    /// Ajuste fino (rampa de gamma, por monitor).
    pub tone: Tone,
    /// Profundidad de color (matriz).
    pub color: ColorDepth,
    /// Ayuda para daltonismo (matriz).
    pub vision: VisionAid,
    /// Cuidado visual (rampa).
    pub eye_care: EyeCare,
}

impl Default for Look {
    fn default() -> Self {
        Self {
            saturation: ranges::SATURATION.default,
            vibrance: ranges::VIBRANCE.default,
            tone: Tone::default(),
            color: ColorDepth::default(),
            vision: VisionAid::default(),
            eye_care: EyeCare::default(),
        }
    }
}

/// Ajuste fino: se aplica con la rampa de gamma de cada monitor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct Tone {
    /// 50–170 %.
    pub brightness: f32,
    /// 50–150 %.
    pub contrast: f32,
    /// 0,50–1,50 (mayor que 1 aclara).
    pub gamma: f32,
    /// −100 (frío) … 100 (cálido).
    pub temperature: f32,
    /// −100…100.
    pub highlights: f32,
    /// −100…100.
    pub shadows: f32,
    /// −100…100.
    pub blacks: f32,
    /// −100…100.
    pub whites: f32,
    /// −100 (frío) … 100 (cálido).
    pub shadow_tint: f32,
    /// −100 (frío) … 100 (cálido).
    pub highlight_tint: f32,
    /// Ganancia R, G, B (0,50–1,50).
    pub gain: [f32; 3],
}

impl Default for Tone {
    fn default() -> Self {
        Self {
            brightness: ranges::BRIGHTNESS.default,
            contrast: ranges::CONTRAST.default,
            gamma: ranges::GAMMA.default,
            temperature: 0.0,
            highlights: 0.0,
            shadows: 0.0,
            blacks: 0.0,
            whites: 0.0,
            shadow_tint: 0.0,
            highlight_tint: 0.0,
            gain: [1.0; 3],
        }
    }
}

/// Fórmula de luminancia: define qué se considera "gris" al saturar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum LumaFormula {
    #[default]
    Rec709,
    Rec601,
    Rec2020,
    Average,
}

impl LumaFormula {
    /// Pesos R, G, B (suman 1).
    pub fn weights(self) -> [f32; 3] {
        match self {
            Self::Rec709 => [0.2126, 0.7152, 0.0722],
            Self::Rec601 => [0.299, 0.587, 0.114],
            Self::Rec2020 => [0.2627, 0.678, 0.0593],
            Self::Average => [1.0 / 3.0; 3],
        }
    }
}

/// Profundidad de color: se aplica con la matriz (todo el escritorio).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ColorDepth {
    pub luma: LumaFormula,
    /// −180…180 grados.
    pub hue_shift_deg: f32,
    /// 0–100 % del giro de tono.
    pub hue_shift_amount: f32,
    /// Tono a oscurecer, 0–360 grados.
    pub darken_hue_deg: f32,
    /// 0–100 %.
    pub darken_amount: f32,
}

impl Default for ColorDepth {
    fn default() -> Self {
        Self {
            luma: LumaFormula::Rec709,
            hue_shift_deg: 0.0,
            hue_shift_amount: 100.0,
            darken_hue_deg: 0.0,
            darken_amount: 0.0,
        }
    }
}

/// Tipo de ayuda para daltonismo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum VisionMode {
    #[default]
    Off,
    /// Rojo débil.
    Protan,
    /// Verde débil.
    Deutan,
    /// Azul débil.
    Tritan,
}

/// Ayuda para daltonismo (fase F5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct VisionAid {
    pub mode: VisionMode,
    /// 0–100 %.
    pub strength: f32,
}

impl Default for VisionAid {
    fn default() -> Self {
        Self {
            mode: VisionMode::Off,
            strength: 100.0,
        }
    }
}

/// Cuidado visual: se suma a cualquier look.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct EyeCare {
    /// 0–100 %: calienta la imagen.
    pub warmth: f32,
    /// 0–80 %: atenúa la imagen.
    pub dim: f32,
}

/// Ajuste propio de un monitor (se compone con el look).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct DisplayTrim {
    /// 0,70–1,30: multiplica la gamma del look.
    pub gamma: f32,
    /// −100…100: se suma a la temperatura del look.
    pub warmth: f32,
    /// Ganancia R, G, B (0,50–1,50).
    pub gain: [f32; 3],
}

impl Default for DisplayTrim {
    fn default() -> Self {
        Self {
            gamma: 1.0,
            warmth: 0.0,
            gain: [1.0; 3],
        }
    }
}

impl Look {
    /// Copia con todos los valores dentro de sus rangos.
    pub fn clamped(&self) -> Self {
        let tone = &self.tone;
        Self {
            saturation: ranges::SATURATION.clamp(self.saturation),
            vibrance: ranges::VIBRANCE.clamp(self.vibrance),
            tone: Tone {
                brightness: ranges::BRIGHTNESS.clamp(tone.brightness),
                contrast: ranges::CONTRAST.clamp(tone.contrast),
                gamma: ranges::GAMMA.clamp(tone.gamma),
                temperature: ranges::TEMPERATURE.clamp(tone.temperature),
                highlights: ranges::TONAL.clamp(tone.highlights),
                shadows: ranges::TONAL.clamp(tone.shadows),
                blacks: ranges::TONAL.clamp(tone.blacks),
                whites: ranges::TONAL.clamp(tone.whites),
                shadow_tint: ranges::TONAL.clamp(tone.shadow_tint),
                highlight_tint: ranges::TONAL.clamp(tone.highlight_tint),
                gain: tone.gain.map(|g| ranges::GAIN.clamp(g)),
            },
            color: ColorDepth {
                luma: self.color.luma,
                hue_shift_deg: ranges::HUE_SHIFT.clamp(self.color.hue_shift_deg),
                hue_shift_amount: ranges::PERCENT.clamp(self.color.hue_shift_amount),
                darken_hue_deg: ranges::HUE.clamp(self.color.darken_hue_deg),
                darken_amount: ranges::PERCENT.clamp(self.color.darken_amount),
            },
            vision: VisionAid {
                mode: self.vision.mode,
                strength: ranges::PERCENT.clamp(self.vision.strength),
            },
            eye_care: EyeCare {
                warmth: ranges::PERCENT.clamp(self.eye_care.warmth),
                dim: ranges::DIM.clamp(self.eye_care.dim),
            },
        }
    }
}

impl DisplayTrim {
    /// Copia con todos los valores dentro de sus rangos.
    pub fn clamped(&self) -> Self {
        Self {
            gamma: ranges::TRIM_GAMMA.clamp(self.gamma),
            warmth: ranges::TEMPERATURE.clamp(self.warmth),
            gain: self.gain.map(|g| ranges::GAIN.clamp(g)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_look_round_trips_through_json() {
        let look = Look::default();
        let json = serde_json::to_string(&look).expect("serializable");
        let back: Look = serde_json::from_str(&json).expect("deserializable");
        assert_eq!(back, look);
    }

    #[test]
    fn missing_fields_take_defaults() {
        let look: Look = serde_json::from_str(r#"{"saturation": 150}"#).expect("válido");
        assert_eq!(look.saturation, 150.0);
        assert_eq!(look.tone, Tone::default());
    }

    #[test]
    fn luma_weights_sum_to_one() {
        for formula in [
            LumaFormula::Rec709,
            LumaFormula::Rec601,
            LumaFormula::Rec2020,
            LumaFormula::Average,
        ] {
            let sum: f32 = formula.weights().iter().sum();
            assert!((sum - 1.0).abs() < 1e-3, "{formula:?}: {sum}");
        }
    }

    #[test]
    fn clamped_keeps_values_in_range() {
        let look = Look {
            saturation: 1000.0,
            tone: Tone {
                gamma: f32::NAN,
                ..Tone::default()
            },
            eye_care: EyeCare {
                dim: 99.0,
                ..EyeCare::default()
            },
            ..Look::default()
        };
        let clamped = look.clamped();
        assert_eq!(clamped.saturation, 300.0);
        assert_eq!(clamped.tone.gamma, 1.0);
        assert_eq!(clamped.eye_care.dim, 80.0);
    }
}
