//! Matriz de color 5×5 de Windows (`MAGCOLOREFFECT`).
//!
//! Convención de **vector fila**: `[r g b a 1] · M`. La fila `i` dice cuánto
//! aporta el canal de entrada `i` a cada canal de salida; los desplazamientos
//! van en la fila 4. Para que un gris siga siendo gris, cada columna del bloque
//! de color tiene que sumar 1.

use crate::model::look::Look;

/// Límite por defecto para los coeficientes del bloque de color.
///
/// Microsoft no documenta ninguno; es una guarda contra valores absurdos que
/// se confirma en hardware real.
pub const DEFAULT_COEFFICIENT_LIMIT: f32 = 16.0;

/// Matriz de color en convención de vector fila.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorMatrix(pub [[f32; 5]; 5]);

impl Default for ColorMatrix {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Resultado de [`ColorMatrix::sanitize`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sanitized {
    pub matrix: ColorMatrix,
    /// Cuánto se conservó del cambio pedido (1 = todo).
    pub kept: f32,
    /// Había valores inválidos (`NaN`, infinitos) y se usó la identidad.
    pub replaced_invalid: bool,
}

impl ColorMatrix {
    /// La matriz que no cambia nada.
    pub const IDENTITY: Self = Self([
        [1.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 0.0, 1.0],
    ]);

    /// Composición: primero se aplica `self` y después `next`.
    #[allow(clippy::needless_range_loop)]
    pub fn then(&self, next: &ColorMatrix) -> ColorMatrix {
        let mut out = [[0.0f32; 5]; 5];
        for i in 0..5 {
            for j in 0..5 {
                out[i][j] = (0..5).map(|k| self.0[i][k] * next.0[k][j]).sum();
            }
        }
        ColorMatrix(out)
    }

    /// Aplica la matriz a un color RGB (componentes de 0 a 1, alfa 1).
    #[allow(clippy::needless_range_loop)]
    pub fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        let input = [rgb[0], rgb[1], rgb[2], 1.0, 1.0];
        let mut out = [0.0f32; 3];
        for j in 0..3 {
            out[j] = (0..5).map(|i| input[i] * self.0[i][j]).sum();
        }
        out
    }

    /// Interpolación lineal elemento a elemento (`t = 0` → `self`).
    pub fn lerp(&self, other: &ColorMatrix, t: f32) -> ColorMatrix {
        let mut out = self.0;
        for (row, other_row) in out.iter_mut().zip(other.0.iter()) {
            for (value, other_value) in row.iter_mut().zip(other_row.iter()) {
                *value += (other_value - *value) * t;
            }
        }
        ColorMatrix(out)
    }

    /// Mayor diferencia absoluta entre dos matrices.
    pub fn max_abs_diff(&self, other: &ColorMatrix) -> f32 {
        self.to_flat()
            .iter()
            .zip(other.to_flat().iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f32::max)
    }

    /// `true` si es la identidad dentro de `tolerance`.
    pub fn is_identity(&self, tolerance: f32) -> bool {
        self.max_abs_diff(&Self::IDENTITY) <= tolerance
    }

    /// `true` si suma o resta un valor fijo a algún canal (fila 4).
    pub fn has_offsets(&self) -> bool {
        self.0[4][..3].iter().any(|value| value.abs() > 1e-6)
    }

    /// Los 25 valores fila por fila (el orden de `MAGCOLOREFFECT`).
    pub fn to_flat(&self) -> [f32; 25] {
        let mut out = [0.0f32; 25];
        for (index, value) in self.0.iter().flatten().enumerate() {
            out[index] = *value;
        }
        out
    }

    /// Construye la matriz a partir de 25 valores fila por fila.
    pub fn from_flat(values: [f32; 25]) -> Self {
        let mut out = [[0.0f32; 5]; 5];
        for (index, value) in values.into_iter().enumerate() {
            out[index / 5][index % 5] = value;
        }
        Self(out)
    }

    /// Deja la matriz lista para Windows:
    ///
    /// - con valores inválidos, devuelve la identidad;
    /// - fuerza la estructura (alfa intacto, columna 4 = `[0, 0, 0, 0, 1]`);
    /// - si algún coeficiente del bloque de color supera `limit`, acerca la
    ///   matriz a la identidad (`I + k·(M − I)`) en vez de recortar cada valor:
    ///   así un gris sigue siendo gris;
    /// - recorta los desplazamientos a `[−1, 1]`.
    #[allow(clippy::needless_range_loop)]
    pub fn sanitize(&self, limit: f32) -> Sanitized {
        if self.0.iter().flatten().any(|value| !value.is_finite()) {
            return Sanitized {
                matrix: Self::IDENTITY,
                kept: 0.0,
                replaced_invalid: true,
            };
        }

        let mut kept = 1.0f32;
        for i in 0..3 {
            for j in 0..3 {
                let identity = if i == j { 1.0 } else { 0.0 };
                let delta = self.0[i][j] - identity;
                if delta == 0.0 {
                    continue;
                }
                // Mayor k tal que |identity + k·delta| ≤ limit.
                let bound = if delta > 0.0 {
                    (limit - identity) / delta
                } else {
                    (limit + identity) / -delta
                };
                kept = kept.min(bound.max(0.0));
            }
        }

        let mut out = Self::IDENTITY.0;
        for i in 0..3 {
            for j in 0..3 {
                let identity = Self::IDENTITY.0[i][j];
                out[i][j] = identity + kept * (self.0[i][j] - identity);
            }
        }
        for j in 0..3 {
            out[4][j] = self.0[4][j].clamp(-1.0, 1.0);
        }

        Sanitized {
            matrix: Self(out),
            kept,
            replaced_invalid: false,
        }
    }
}

/// Saturación con factor `factor` (1 = sin cambio, 0 = escala de grises)
/// respecto de la luminancia con pesos `weights` (suman 1).
pub fn saturation(factor: f32, weights: [f32; 3]) -> ColorMatrix {
    let mut out = ColorMatrix::IDENTITY.0;
    for (i, row) in out.iter_mut().enumerate().take(3) {
        for (j, value) in row.iter_mut().enumerate().take(3) {
            *value = weights[i] * (1.0 - factor) + if i == j { factor } else { 0.0 };
        }
    }
    ColorMatrix(out)
}

/// Giro de tono en grados (matriz `hueRotate` del W3C, traspuesta para la
/// convención de vector fila). Conserva el gris.
pub fn hue_rotation(degrees: f32) -> ColorMatrix {
    let (sin, cos) = degrees.to_radians().sin_cos();
    // Filas de la matriz del W3C (convención de vector columna).
    let w3c = [
        [
            0.213 + cos * 0.787 - sin * 0.213,
            0.715 - cos * 0.715 - sin * 0.715,
            0.072 - cos * 0.072 + sin * 0.928,
        ],
        [
            0.213 - cos * 0.213 + sin * 0.143,
            0.715 + cos * 0.285 + sin * 0.140,
            0.072 - cos * 0.072 - sin * 0.283,
        ],
        [
            0.213 - cos * 0.213 - sin * 0.787,
            0.715 - cos * 0.715 + sin * 0.715,
            0.072 + cos * 0.928 + sin * 0.072,
        ],
    ];
    let mut out = ColorMatrix::IDENTITY.0;
    for (i, row) in out.iter_mut().enumerate().take(3) {
        for (j, value) in row.iter_mut().enumerate().take(3) {
            *value = w3c[j][i];
        }
    }
    ColorMatrix(out)
}

/// Invierte los colores (`1 − valor`).
#[allow(clippy::needless_range_loop)]
pub fn invert() -> ColorMatrix {
    let mut out = ColorMatrix::IDENTITY.0;
    for i in 0..3 {
        out[i][i] = -1.0;
        out[4][i] = 1.0;
    }
    ColorMatrix(out)
}

/// Factor de croma a partir de la saturación (0–300 %) y, si no hay vibrance
/// del driver en ningún monitor, de la vibrance emulada (0–100, 50 = neutro).
pub fn chroma_factor(saturation_pct: f32, emulated_vibrance: Option<f32>) -> f32 {
    let saturation = (saturation_pct / 100.0).clamp(0.0, 3.0);
    let vibrance = emulated_vibrance.map_or(1.0, |v| (v / 50.0).clamp(0.0, 2.0));
    saturation * vibrance
}

/// Matriz de un look: giro de tono → saturación → límites de seguridad.
///
/// (Oscurecer un tono y las ayudas para daltonismo llegan en la fase F5.)
pub fn build_matrix(look: &Look, emulated_vibrance: Option<f32>, limit: f32) -> Sanitized {
    let look = look.clamped();
    let weights = look.color.luma.weights();

    let amount = look.color.hue_shift_amount / 100.0;
    let hue = ColorMatrix::IDENTITY.lerp(&hue_rotation(look.color.hue_shift_deg), amount);
    let chroma = saturation(chroma_factor(look.saturation, emulated_vibrance), weights);

    hue.then(&chroma).sanitize(limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::look::{ColorDepth, LumaFormula};
    use proptest::prelude::*;

    const EPS: f32 = 1e-4;

    fn assert_gray(matrix: &ColorMatrix, gray: f32) {
        let out = matrix.apply([gray; 3]);
        for channel in out {
            assert!(
                (channel - gray).abs() < 1e-3,
                "el gris {gray} salió como {out:?}"
            );
        }
    }

    #[test]
    fn identity_is_neutral_for_composition() {
        let m = saturation(1.7, LumaFormula::Rec709.weights());
        assert!(ColorMatrix::IDENTITY.then(&m).max_abs_diff(&m) < EPS);
        assert!(m.then(&ColorMatrix::IDENTITY).max_abs_diff(&m) < EPS);
    }

    #[test]
    fn neutral_look_builds_identity() {
        let built = build_matrix(&Look::default(), None, DEFAULT_COEFFICIENT_LIMIT);
        assert!(built.matrix.is_identity(EPS), "{:?}", built.matrix);
        assert_eq!(built.kept, 1.0);
    }

    #[test]
    fn zero_saturation_is_grayscale() {
        let m = saturation(0.0, LumaFormula::Rec709.weights());
        let out = m.apply([1.0, 0.0, 0.0]);
        assert!((out[0] - 0.2126).abs() < EPS && (out[1] - 0.2126).abs() < EPS);
    }

    #[test]
    fn saturation_pushes_colors_away_from_gray() {
        let m = saturation(2.0, LumaFormula::Rec709.weights());
        let out = m.apply([0.6, 0.4, 0.4]);
        assert!(out[0] > 0.6 && out[1] < 0.4, "{out:?}");
    }

    #[test]
    fn hue_rotation_full_turn_is_identity() {
        assert!(hue_rotation(0.0).is_identity(1e-3));
        assert!(hue_rotation(360.0).is_identity(1e-3));
        assert!(!hue_rotation(120.0).is_identity(1e-2));
    }

    #[test]
    fn flat_round_trip_keeps_row_order() {
        let m = saturation(1.5, LumaFormula::Rec709.weights());
        let flat = m.to_flat();
        assert_eq!(flat[1], m.0[0][1]);
        assert_eq!(flat[5], m.0[1][0]);
        assert_eq!(ColorMatrix::from_flat(flat), m);
    }

    #[test]
    fn invert_maps_black_to_white() {
        let out = invert().apply([0.0, 0.25, 1.0]);
        assert!((out[0] - 1.0).abs() < EPS && (out[1] - 0.75).abs() < EPS && out[2].abs() < EPS);
        assert!(invert().has_offsets());
    }

    #[test]
    fn invalid_values_become_identity() {
        let mut m = ColorMatrix::IDENTITY;
        m.0[1][2] = f32::NAN;
        let sanitized = m.sanitize(DEFAULT_COEFFICIENT_LIMIT);
        assert!(sanitized.replaced_invalid);
        assert_eq!(sanitized.matrix, ColorMatrix::IDENTITY);
    }

    #[test]
    fn sanitize_scales_toward_identity() {
        let m = saturation(40.0, LumaFormula::Rec709.weights());
        let sanitized = m.sanitize(4.0);
        assert!(sanitized.kept < 1.0);
        let max = sanitized.matrix.0[..3]
            .iter()
            .flat_map(|row| row[..3].iter())
            .fold(0.0f32, |acc, v| acc.max(v.abs()));
        assert!(max <= 4.0 + EPS, "{max}");
        assert_gray(&sanitized.matrix, 0.5);
    }

    #[test]
    fn maximum_saturation_fits_default_limit() {
        let look = Look {
            saturation: 300.0,
            ..Look::default()
        };
        let built = build_matrix(&look, Some(100.0), DEFAULT_COEFFICIENT_LIMIT);
        assert_eq!(
            built.kept, 1.0,
            "a 300 % con vibrance emulada no hace falta recortar"
        );
    }

    proptest! {
        #[test]
        fn any_look_keeps_gray_gray(
            saturation_pct in 0.0f32..=300.0,
            vibrance in 0.0f32..=100.0,
            hue in -180.0f32..=180.0,
            amount in 0.0f32..=100.0,
            luma in prop_oneof![
                Just(LumaFormula::Rec709), Just(LumaFormula::Rec601),
                Just(LumaFormula::Rec2020), Just(LumaFormula::Average),
            ],
            gray in 0.0f32..=1.0,
        ) {
            let look = Look {
                saturation: saturation_pct,
                color: ColorDepth {
                    luma,
                    hue_shift_deg: hue,
                    hue_shift_amount: amount,
                    ..ColorDepth::default()
                },
                ..Look::default()
            };
            let built = build_matrix(&look, Some(vibrance), DEFAULT_COEFFICIENT_LIMIT);
            prop_assert!(built.matrix.0.iter().flatten().all(|v| v.is_finite()));
            let out = built.matrix.apply([gray; 3]);
            for channel in out {
                prop_assert!((channel - gray).abs() < 2e-3, "gris {} → {:?}", gray, out);
            }
        }

        #[test]
        fn sanitize_respects_any_limit(factor in -50.0f32..=50.0, limit in 1.5f32..=16.0) {
            let m = saturation(factor, LumaFormula::Rec709.weights());
            let sanitized = m.sanitize(limit);
            for row in &sanitized.matrix.0[..3] {
                for value in &row[..3] {
                    prop_assert!(value.abs() <= limit + 1e-3, "{} > {}", value, limit);
                }
            }
            prop_assert!(sanitized.kept >= 0.0 && sanitized.kept <= 1.0);
        }
    }
}
