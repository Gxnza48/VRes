//! Rampas de gamma: la tabla de 3 × 256 valores de 16 bits que la placa de
//! video aplica a cada monitor justo antes de enviar la imagen.
//!
//! VRes nunca reemplaza la rampa original de un monitor (que puede venir de un
//! calibrador): construye su curva y la **compone sobre** la original.

use super::kelvin;
use crate::model::look::{DisplayTrim, EyeCare, Tone};

/// Cantidad de entradas por canal.
pub const RAMP_SIZE: usize = 256;

/// Rampa de gamma: canales R, G y B (mismo orden y tamaño que `WORD[3][256]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GammaRamp {
    pub channels: [[u16; RAMP_SIZE]; 3],
}

impl Default for GammaRamp {
    fn default() -> Self {
        Self::identity()
    }
}

/// Regla con la que Windows acepta o rechaza una rampa: el byte alto de cada
/// entrada no puede alejarse de su índice más que `max_deviation`.
///
/// El límite de Windows es 128 (valor `GdiIcmGammaRange` del registro); VRes
/// deja un margen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RampRule {
    pub max_deviation: u16,
}

impl RampRule {
    /// Límite por defecto de Windows.
    pub const WINDOWS_DEFAULT: Self = Self { max_deviation: 128 };
    /// Límite que usa VRes (con margen).
    pub const VRES: Self = Self { max_deviation: 120 };
}

/// Primera entrada que no cumple la regla.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RampViolation {
    pub channel: usize,
    pub index: usize,
    /// Cuánto se aleja (en unidades de 8 bits).
    pub deviation: u16,
}

impl GammaRamp {
    /// La rampa que no cambia nada (`i × 257`).
    pub fn identity() -> Self {
        let mut channel = [0u16; RAMP_SIZE];
        for (index, value) in channel.iter_mut().enumerate() {
            *value = (index as u16) * 257;
        }
        Self {
            channels: [channel; 3],
        }
    }

    /// Construye la rampa evaluando una curva por canal (`x` e `y` en 0–1).
    pub fn from_fn(curve: impl Fn(usize, f64) -> f64) -> Self {
        let mut channels = [[0u16; RAMP_SIZE]; 3];
        for (channel, values) in channels.iter_mut().enumerate() {
            for (index, value) in values.iter_mut().enumerate() {
                let x = index as f64 / (RAMP_SIZE - 1) as f64;
                *value = to_u16(curve(channel, x));
            }
        }
        Self { channels }
    }

    /// `true` si es exactamente la identidad.
    pub fn is_identity(&self) -> bool {
        *self == Self::identity()
    }

    /// `true` si ningún canal baja en ningún punto.
    pub fn is_monotonic(&self) -> bool {
        self.channels
            .iter()
            .all(|channel| channel.windows(2).all(|pair| pair[0] <= pair[1]))
    }

    /// Mayor diferencia entre dos rampas, entrada por entrada.
    pub fn max_abs_diff(&self, other: &GammaRamp) -> u16 {
        self.channels
            .iter()
            .flatten()
            .zip(other.channels.iter().flatten())
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap_or(0)
    }

    /// Mayor alejamiento de la identidad, en unidades de 8 bits.
    pub fn max_deviation(&self) -> u16 {
        self.channels
            .iter()
            .flat_map(|channel| channel.iter().enumerate())
            .map(|(index, value)| (value >> 8).abs_diff(index as u16))
            .max()
            .unwrap_or(0)
    }

    /// Verifica la regla de Windows.
    pub fn validate(&self, rule: RampRule) -> Result<(), RampViolation> {
        for (channel, values) in self.channels.iter().enumerate() {
            for (index, value) in values.iter().enumerate() {
                let deviation = (value >> 8).abs_diff(index as u16);
                if deviation > rule.max_deviation {
                    return Err(RampViolation {
                        channel,
                        index,
                        deviation,
                    });
                }
            }
        }
        Ok(())
    }

    /// Valor de la rampa en `y` (0–1), interpolando entre entradas.
    fn sample(&self, channel: usize, y: f64) -> f64 {
        let position = y.clamp(0.0, 1.0) * (RAMP_SIZE - 1) as f64;
        let low = position.floor() as usize;
        let high = (low + 1).min(RAMP_SIZE - 1);
        let t = position - low as f64;
        let values = &self.channels[channel];
        f64::from(values[low]) + (f64::from(values[high]) - f64::from(values[low])) * t
    }

    /// Compone `self` sobre `base`: primero `self`, después la curva de `base`
    /// (así se conserva la calibración del monitor).
    pub fn compose_over(&self, base: &GammaRamp) -> GammaRamp {
        let mut channels = [[0u16; RAMP_SIZE]; 3];
        for (channel, values) in channels.iter_mut().enumerate() {
            for (index, value) in values.iter_mut().enumerate() {
                let y = f64::from(self.channels[channel][index]) / 65535.0;
                *value = base.sample(channel, y).round().clamp(0.0, 65535.0) as u16;
            }
        }
        GammaRamp { channels }
    }

    /// Mezcla con `base`: `k = 0` → `base`, `k = 1` → `self`.
    pub fn blend_toward(&self, base: &GammaRamp, k: f64) -> GammaRamp {
        let k = k.clamp(0.0, 1.0);
        let mut channels = base.channels;
        for (channel, values) in channels.iter_mut().enumerate() {
            for (index, value) in values.iter_mut().enumerate() {
                let from = f64::from(base.channels[channel][index]);
                let to = f64::from(self.channels[channel][index]);
                *value = (from + (to - from) * k).round().clamp(0.0, 65535.0) as u16;
            }
        }
        GammaRamp { channels }
    }

    /// Fuerza que ningún canal baje (máximo acumulado).
    pub fn make_monotonic(&mut self) {
        for channel in &mut self.channels {
            let mut running = 0u16;
            for value in channel.iter_mut() {
                running = running.max(*value);
                *value = running;
            }
        }
    }

    /// Bytes en el orden de `WORD[3][256]` (little-endian): para guardarla.
    pub fn to_le_bytes(&self) -> Vec<u8> {
        self.channels
            .iter()
            .flatten()
            .flat_map(|value| value.to_le_bytes())
            .collect()
    }

    /// Lee una rampa guardada con [`GammaRamp::to_le_bytes`].
    pub fn from_le_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 3 * RAMP_SIZE * 2 {
            return None;
        }
        let mut channels = [[0u16; RAMP_SIZE]; 3];
        let (pairs, _) = bytes.as_chunks::<2>();
        for (index, pair) in pairs.iter().enumerate() {
            channels[index / RAMP_SIZE][index % RAMP_SIZE] = u16::from_le_bytes(*pair);
        }
        Some(Self { channels })
    }
}

fn to_u16(value: f64) -> u16 {
    (value.clamp(0.0, 1.0) * 65535.0).round() as u16
}

/// Parámetros de la curva de un monitor, ya en unidades internas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RampParams {
    /// Mayor que 1 aclara.
    pub gamma: f64,
    /// Multiplicador (1 = sin cambio).
    pub brightness: f64,
    /// Multiplicador alrededor del gris medio (1 = sin cambio).
    pub contrast: f64,
    /// −1…1.
    pub shadows: f64,
    /// −1…1.
    pub highlights: f64,
    /// −1…1.
    pub blacks: f64,
    /// −1…1.
    pub whites: f64,
    /// −1 (frío) … 1 (cálido).
    pub shadow_tint: f64,
    /// −1 (frío) … 1 (cálido).
    pub highlight_tint: f64,
    /// Ganancia final por canal (temperatura × ganancia × atenuación).
    pub channel_gain: [f64; 3],
}

impl RampParams {
    /// Combina el ajuste fino del look, el cuidado visual y el ajuste propio
    /// del monitor.
    pub fn new(tone: &Tone, eye_care: &EyeCare, trim: &DisplayTrim) -> Self {
        let percent = |value: f32| f64::from(value) / 100.0;
        let temperature =
            f64::from(tone.temperature) + f64::from(trim.warmth) + f64::from(eye_care.warmth);
        let temperature_gain = kelvin::rgb_gain(kelvin::kelvin_from_temperature(temperature));
        let dim = 1.0 - percent(eye_care.dim).clamp(0.0, 0.8);
        let channel_gain = [0, 1, 2]
            .map(|i| temperature_gain[i] * f64::from(tone.gain[i]) * f64::from(trim.gain[i]) * dim);

        Self {
            gamma: (f64::from(tone.gamma) * f64::from(trim.gamma)).clamp(0.3, 2.0),
            brightness: percent(tone.brightness),
            contrast: percent(tone.contrast),
            shadows: percent(tone.shadows),
            highlights: percent(tone.highlights),
            blacks: percent(tone.blacks),
            whites: percent(tone.whites),
            shadow_tint: percent(tone.shadow_tint),
            highlight_tint: percent(tone.highlight_tint),
            channel_gain,
        }
    }

    /// `true` si la curva no cambia nada.
    pub fn is_neutral(&self) -> bool {
        const EPS: f64 = 1e-6;
        let near = |value: f64, target: f64| (value - target).abs() < EPS;
        near(self.gamma, 1.0)
            && near(self.brightness, 1.0)
            && near(self.contrast, 1.0)
            && near(self.shadows, 0.0)
            && near(self.highlights, 0.0)
            && near(self.blacks, 0.0)
            && near(self.whites, 0.0)
            && near(self.shadow_tint, 0.0)
            && near(self.highlight_tint, 0.0)
            && self.channel_gain.iter().all(|gain| near(*gain, 1.0))
    }

    /// Valor de la curva para el canal `channel` en `x` (0–1).
    pub fn evaluate(&self, channel: usize, x: f64) -> f64 {
        let mut y = x.clamp(0.0, 1.0).powf(1.0 / self.gamma);
        y = (y - 0.5) * self.contrast + 0.5;
        y *= self.brightness;

        // Las regiones se miden sobre la imagen ya ajustada: cada control actúa
        // donde dice (sombras abajo, altas luces arriba), sin pisarse.
        let base = y.clamp(0.0, 1.0);
        let low = 6.75 * base * (1.0 - base).powi(2); // máximo 1 en 1/3
        let high = 6.75 * base.powi(2) * (1.0 - base); // máximo 1 en 2/3
        y += 0.145 * (self.shadows * low + self.highlights * high);
        y += 0.10 * (self.blacks * (1.0 - base).powi(3) + self.whites * base.powi(3));

        // Tintes: cálido suma rojo y resta azul; frío, al revés.
        let lean = [1.0, 0.0, -1.0][channel];
        y += lean * 0.08 * (self.shadow_tint * low + self.highlight_tint * high);

        (y * self.channel_gain[channel]).clamp(0.0, 1.0)
    }
}

/// Resultado de [`build_ramp`].
#[derive(Debug, Clone, PartialEq)]
pub struct BuiltRamp {
    /// Rampa final, lista para escribir.
    pub ramp: GammaRamp,
    /// Cuánto se conservó de la curva pedida (1 = toda; menos si hubo que
    /// acercarla a la original para que Windows la acepte).
    pub kept: f64,
}

/// Construye la rampa de un monitor a partir de sus parámetros y de su rampa
/// original, respetando la regla de Windows.
///
/// Si la curva es neutra devuelve exactamente la original.
pub fn build_ramp(params: &RampParams, original: &GammaRamp, rule: RampRule) -> BuiltRamp {
    if params.is_neutral() {
        return BuiltRamp {
            ramp: original.clone(),
            kept: 1.0,
        };
    }

    let mut curve = GammaRamp::from_fn(|channel, x| params.evaluate(channel, x));
    curve.make_monotonic();
    let mut composed = curve.compose_over(original);
    composed.make_monotonic();

    if composed.validate(rule).is_ok() {
        return BuiltRamp {
            ramp: composed,
            kept: 1.0,
        };
    }

    // Búsqueda binaria del mayor k que Windows acepta.
    let (mut low, mut high) = (0.0f64, 1.0f64);
    for _ in 0..16 {
        let mid = (low + high) / 2.0;
        if composed.blend_toward(original, mid).validate(rule).is_ok() {
            low = mid;
        } else {
            high = mid;
        }
    }
    BuiltRamp {
        ramp: composed.blend_toward(original, low),
        kept: low,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn params(tone: Tone) -> RampParams {
        RampParams::new(&tone, &EyeCare::default(), &DisplayTrim::default())
    }

    #[test]
    fn identity_has_expected_endpoints() {
        let identity = GammaRamp::identity();
        assert_eq!(identity.channels[0][0], 0);
        assert_eq!(identity.channels[2][255], 65535);
        assert!(identity.is_monotonic());
        assert_eq!(identity.max_deviation(), 0);
        assert!(identity.validate(RampRule::VRES).is_ok());
    }

    #[test]
    fn neutral_params_return_the_original_exactly() {
        let mut calibrated = GammaRamp::identity();
        calibrated.channels[0][10] += 100;
        let built = build_ramp(&params(Tone::default()), &calibrated, RampRule::VRES);
        assert_eq!(built.ramp, calibrated);
        assert_eq!(built.kept, 1.0);
    }

    #[test]
    fn gamma_above_one_brightens_midtones() {
        let tone = Tone {
            gamma: 1.3,
            ..Tone::default()
        };
        let built = build_ramp(&params(tone), &GammaRamp::identity(), RampRule::VRES);
        assert!(built.ramp.channels[1][128] > GammaRamp::identity().channels[1][128]);
        assert_eq!(built.ramp.channels[1][0], 0);
        assert_eq!(built.ramp.channels[1][255], 65535);
    }

    #[test]
    fn warm_temperature_lowers_blue() {
        let tone = Tone {
            temperature: 80.0,
            ..Tone::default()
        };
        let built = build_ramp(&params(tone), &GammaRamp::identity(), RampRule::VRES);
        let [red, _, blue] = built.ramp.channels;
        assert!(blue[200] < red[200]);
    }

    #[test]
    fn extreme_curves_are_fitted_to_the_windows_rule() {
        let tone = Tone {
            brightness: 170.0,
            gamma: 1.5,
            shadows: 100.0,
            blacks: 100.0,
            ..Tone::default()
        };
        let built = build_ramp(&params(tone), &GammaRamp::identity(), RampRule::VRES);
        assert!(built.ramp.validate(RampRule::VRES).is_ok());
        assert!(built.kept < 1.0);
        assert!(built.ramp.is_monotonic());
    }

    #[test]
    fn compose_over_identity_keeps_the_curve() {
        let curve = GammaRamp::from_fn(|_, x| x.powf(0.8));
        let composed = curve.compose_over(&GammaRamp::identity());
        assert!(composed.max_abs_diff(&curve) <= 1);
    }

    #[test]
    fn bytes_round_trip() {
        let ramp = GammaRamp::from_fn(|channel, x| x * (0.8 + 0.1 * channel as f64));
        let bytes = ramp.to_le_bytes();
        assert_eq!(bytes.len(), 1536);
        assert_eq!(GammaRamp::from_le_bytes(&bytes), Some(ramp));
        assert_eq!(GammaRamp::from_le_bytes(&bytes[1..]), None);
    }

    fn arb_tone() -> impl Strategy<Value = Tone> {
        (
            (
                50.0f32..=170.0,
                50.0f32..=150.0,
                0.5f32..=1.5,
                -100.0f32..=100.0,
            ),
            (
                -100.0f32..=100.0,
                -100.0f32..=100.0,
                -100.0f32..=100.0,
                -100.0f32..=100.0,
            ),
            (-100.0f32..=100.0, -100.0f32..=100.0),
            (0.5f32..=1.5, 0.5f32..=1.5, 0.5f32..=1.5),
        )
            .prop_map(
                |(
                    (brightness, contrast, gamma, temperature),
                    (highlights, shadows, blacks, whites),
                    (shadow_tint, highlight_tint),
                    (r, g, b),
                )| Tone {
                    brightness,
                    contrast,
                    gamma,
                    temperature,
                    highlights,
                    shadows,
                    blacks,
                    whites,
                    shadow_tint,
                    highlight_tint,
                    gain: [r, g, b],
                },
            )
    }

    proptest! {
        #[test]
        fn any_tone_gives_a_valid_monotonic_ramp(
            tone in arb_tone(),
            warmth in 0.0f32..=100.0,
            dim in 0.0f32..=80.0,
        ) {
            let eye = EyeCare { warmth, dim };
            let p = RampParams::new(&tone, &eye, &DisplayTrim::default());
            let built = build_ramp(&p, &GammaRamp::identity(), RampRule::VRES);
            prop_assert!(built.ramp.is_monotonic());
            prop_assert!(built.ramp.validate(RampRule::VRES).is_ok());
            prop_assert!((0.0..=1.0).contains(&built.kept));
        }

        #[test]
        fn evaluate_stays_in_range(tone in arb_tone(), x in 0.0f64..=1.0, channel in 0usize..3) {
            let y = params(tone).evaluate(channel, x);
            prop_assert!((0.0..=1.0).contains(&y));
        }
    }
}
