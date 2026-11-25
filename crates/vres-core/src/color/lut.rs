//! LUT 3D de un look (formato `.cube`), para videos y capturas.
//!
//! Las grabaciones de juegos no ven lo que hace VRes: la matriz de color la
//! aplica Windows al componer el escritorio, y la rampa y la vibrance del
//! driver, la salida de video. Esta tabla reproduce lo mismo que llega al
//! monitor (primero la matriz, después la rampa) para aplicarlo a un video en
//! un editor o con `ffmpeg -vf lut3d`.

use std::fmt::Write as _;

use crate::color::matrix::{DEFAULT_COEFFICIENT_LIMIT, build_matrix};
use crate::color::ramp::{GammaRamp, RAMP_SIZE, RampParams, RampRule, build_ramp};
use crate::model::look::{DisplayTrim, Look};

/// Puntos por lado admitidos (el formato admite de 2 a 256; más allá de 65 el
/// archivo crece mucho sin ganar nada visible).
pub const LUT_SIZES: std::ops::RangeInclusive<usize> = 2..=65;

/// Qué hacer con la vibrance del look: la del driver (NVIDIA o AMD) no se puede
/// reproducir fuera de la placa de video.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LutVibrance {
    /// Emulada en la matriz, como hace VRes en una PC sin vibrance de driver.
    Emulated,
    /// Sin vibrance: solo saturación, tono y color.
    Off,
}

/// Tabla de `size³` colores. El rojo varía más rápido, después el verde y
/// por último el azul (el orden de `.cube`).
#[derive(Debug, Clone, PartialEq)]
pub struct Lut {
    pub size: usize,
    pub values: Vec<[f32; 3]>,
}

/// Lee una rampa en `x` (0 a 1) interpolando entre sus 256 entradas.
fn sample(channel: &[u16; RAMP_SIZE], x: f32) -> f32 {
    let position = x.clamp(0.0, 1.0) * (RAMP_SIZE - 1) as f32;
    let index = (position.floor() as usize).min(RAMP_SIZE - 1);
    let next = (index + 1).min(RAMP_SIZE - 1);
    let t = position - index as f32;
    let value = f32::from(channel[index]) * (1.0 - t) + f32::from(channel[next]) * t;
    value / f32::from(u16::MAX)
}

/// La tabla de un look. `size` se acota a [`LUT_SIZES`].
pub fn look_lut(look: &Look, size: usize, vibrance: LutVibrance) -> Lut {
    let size = size.clamp(*LUT_SIZES.start(), *LUT_SIZES.end());
    let look = look.clamped();
    let emulated = match vibrance {
        LutVibrance::Emulated => Some(look.vibrance),
        LutVibrance::Off => None,
    };
    let matrix = build_matrix(&look, emulated, DEFAULT_COEFFICIENT_LIMIT).matrix;
    // Sobre una rampa neutra y con el mismo límite que acepta Windows.
    let params = RampParams::new(&look.tone, &look.eye_care, &DisplayTrim::default());
    let ramp = build_ramp(&params, &GammaRamp::identity(), RampRule::VRES).ramp;

    let step = 1.0 / (size - 1) as f32;
    let mut values = Vec::with_capacity(size * size * size);
    for blue in 0..size {
        for green in 0..size {
            for red in 0..size {
                let input = [red as f32 * step, green as f32 * step, blue as f32 * step];
                let mixed = matrix.apply(input);
                values.push([
                    sample(&ramp.channels[0], mixed[0]),
                    sample(&ramp.channels[1], mixed[1]),
                    sample(&ramp.channels[2], mixed[2]),
                ]);
            }
        }
    }
    Lut { size, values }
}

impl Lut {
    /// El archivo `.cube` (texto, una fila por color).
    pub fn to_cube(&self, title: &str) -> String {
        let title: String = title
            .chars()
            .filter(|c| *c != '"' && !c.is_control())
            .collect();
        let mut out = String::with_capacity(self.values.len() * 27 + 128);
        let _ = writeln!(out, "TITLE \"{title}\"");
        let _ = writeln!(out, "LUT_3D_SIZE {}", self.size);
        let _ = writeln!(out, "DOMAIN_MIN 0.0 0.0 0.0");
        let _ = writeln!(out, "DOMAIN_MAX 1.0 1.0 1.0");
        for [r, g, b] in &self.values {
            let _ = writeln!(out, "{r:.6} {g:.6} {b:.6}");
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::look::Tone;

    fn index(size: usize, r: usize, g: usize, b: usize) -> usize {
        r + g * size + b * size * size
    }

    #[test]
    fn a_neutral_look_gives_the_identity() {
        let lut = look_lut(&Look::default(), 9, LutVibrance::Emulated);
        assert_eq!(lut.values.len(), 9 * 9 * 9);
        for b in 0..9 {
            for g in 0..9 {
                for r in 0..9 {
                    let got = lut.values[index(9, r, g, b)];
                    let want = [r, g, b].map(|v| v as f32 / 8.0);
                    for channel in 0..3 {
                        assert!(
                            (got[channel] - want[channel]).abs() < 1e-3,
                            "{want:?} → {got:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn saturation_keeps_grays_and_pushes_colors() {
        let look = Look {
            saturation: 180.0,
            ..Look::default()
        };
        let lut = look_lut(&look, 5, LutVibrance::Off);
        let gray = lut.values[index(5, 2, 2, 2)];
        for channel in gray {
            assert!((channel - 0.5).abs() < 1e-3, "el gris cambió: {gray:?}");
        }
        // Un rojo apagado (0.75, 0.5, 0.5) se aleja del gris.
        let muted = lut.values[index(5, 3, 2, 2)];
        assert!(muted[0] > 0.75 && muted[1] < 0.5, "{muted:?}");
    }

    #[test]
    fn emulated_vibrance_adds_color_and_off_ignores_it() {
        let look = Look {
            vibrance: 100.0,
            ..Look::default()
        };
        let emulated = look_lut(&look, 5, LutVibrance::Emulated);
        let off = look_lut(&look, 5, LutVibrance::Off);
        let muted = index(5, 3, 2, 2);
        assert!(emulated.values[muted][0] > off.values[muted][0]);
        assert!(
            (off.values[muted][0] - 0.75).abs() < 1e-3,
            "sin vibrance no cambia"
        );
    }

    #[test]
    fn the_tone_curve_is_applied_after_the_matrix() {
        let look = Look {
            tone: Tone {
                brightness: 120.0,
                ..Tone::default()
            },
            ..Look::default()
        };
        let lut = look_lut(&look, 3, LutVibrance::Off);
        let mid = lut.values[index(3, 1, 1, 1)];
        assert!(mid[0] > 0.55, "el brillo sube los medios: {mid:?}");
        let white = lut.values[index(3, 2, 2, 2)];
        assert!(white[0] <= 1.0);
    }

    #[test]
    fn the_size_is_kept_in_range_and_the_cube_has_one_line_per_color() {
        assert_eq!(look_lut(&Look::default(), 1, LutVibrance::Off).size, 2);
        assert_eq!(look_lut(&Look::default(), 999, LutVibrance::Off).size, 65);
        let cube = look_lut(&Look::default(), 4, LutVibrance::Off).to_cube("Mi \"look\"");
        let lines: Vec<&str> = cube.lines().collect();
        assert_eq!(lines[0], "TITLE \"Mi look\"");
        assert_eq!(lines[1], "LUT_3D_SIZE 4");
        assert_eq!(lines.len(), 4 + 4 * 4 * 4);
        assert_eq!(lines[4], "0.000000 0.000000 0.000000");
        // El segundo color varía solo en rojo.
        assert!(lines[5].starts_with("0.333"));
        assert!(lines[5].ends_with(" 0.000000 0.000000"));
    }
}
