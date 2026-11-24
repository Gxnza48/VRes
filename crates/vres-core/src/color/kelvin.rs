//! Temperatura de color: de grados Kelvin a ganancias R, G, B.

/// Blanco de referencia (D65 aproximado): ganancias 1, 1, 1.
pub const NEUTRAL_KELVIN: f64 = 6500.0;

/// Kelvin por cada punto del deslizador de temperatura (±100 → 4000–9000 K).
const KELVIN_PER_STEP: f64 = 25.0;

/// Convierte el deslizador de temperatura (−100 frío … 100 cálido) a Kelvin.
pub fn kelvin_from_temperature(temperature: f64) -> f64 {
    (NEUTRAL_KELVIN - temperature * KELVIN_PER_STEP).clamp(1500.0, 15000.0)
}

/// Color aproximado del cuerpo negro a `kelvin` grados, en 0–255
/// (aproximación de Tanner Helland a los datos de Mitchell Charity).
fn blackbody_rgb(kelvin: f64) -> [f64; 3] {
    let t = kelvin.clamp(1000.0, 40000.0) / 100.0;
    let red = if t <= 66.0 {
        255.0
    } else {
        329.698_727_446 * (t - 60.0).powf(-0.133_204_759_2)
    };
    let green = if t <= 66.0 {
        99.470_802_586_1 * t.ln() - 161.119_568_166_1
    } else {
        288.122_169_528_3 * (t - 60.0).powf(-0.075_514_849_2)
    };
    let blue = if t >= 66.0 {
        255.0
    } else if t <= 19.0 {
        0.0
    } else {
        138.517_731_223_1 * (t - 10.0).ln() - 305.044_792_730_7
    };
    [red, green, blue].map(|value| value.clamp(0.0, 255.0))
}

/// Ganancias R, G, B para llevar el blanco de 6500 K a `kelvin` grados.
///
/// Normalizadas para que el canal más alto valga 1: nunca aclaran, así no se
/// recortan las luces.
pub fn rgb_gain(kelvin: f64) -> [f64; 3] {
    let reference = blackbody_rgb(NEUTRAL_KELVIN);
    let target = blackbody_rgb(kelvin);
    let relative = [0, 1, 2].map(|i| target[i] / reference[i].max(1e-6));
    let max = relative.iter().copied().fold(f64::MIN, f64::max).max(1e-6);
    relative.map(|value| value / max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_temperature_is_unity_gain() {
        let gain = rgb_gain(kelvin_from_temperature(0.0));
        for channel in gain {
            assert!((channel - 1.0).abs() < 1e-9, "{gain:?}");
        }
    }

    #[test]
    fn warm_lowers_blue_and_cool_lowers_red() {
        let warm = rgb_gain(kelvin_from_temperature(100.0));
        assert!(warm[0] > warm[1] && warm[1] > warm[2], "{warm:?}");
        let cool = rgb_gain(kelvin_from_temperature(-100.0));
        assert!(cool[2] > cool[1] && cool[1] > cool[0], "{cool:?}");
    }

    #[test]
    fn gains_never_exceed_one() {
        for step in -100..=100 {
            let gain = rgb_gain(kelvin_from_temperature(f64::from(step)));
            assert!(
                gain.iter().all(|g| (0.0..=1.0 + 1e-9).contains(g)),
                "{step}: {gain:?}"
            );
        }
    }
}
