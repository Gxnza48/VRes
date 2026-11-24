//! Dark Boost y Zoom: se usan manteniendo una tecla o un botón del mouse (o
//! alternando), solo con un juego al frente salvo que se pida lo contrario, y
//! vienen **apagados**. Pueden considerarse ventaja en algunos juegos o
//! torneos: la interfaz lo avisa.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::color::matrix::{ColorMatrix, invert};
use crate::model::look::Tone;
use crate::model::ranges;

/// Mantener o alternar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum HoldMode {
    /// Activo mientras se mantiene apretado.
    #[default]
    Hold,
    /// Cada pulsación prende o apaga.
    Toggle,
}

/// Botones del mouse que se pueden usar (el izquierdo no: es el de disparar).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum MouseButton {
    Right,
    Middle,
    /// Botón lateral "atrás" (Mouse 4).
    Back,
    /// Botón lateral "adelante" (Mouse 5).
    Forward,
}

impl MouseButton {
    /// Tecla virtual de Windows.
    pub fn virtual_key(self) -> u32 {
        match self {
            Self::Right => 0x02,
            Self::Middle => 0x04,
            Self::Back => 0x05,
            Self::Forward => 0x06,
        }
    }
}

/// Qué hay que apretar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum Binding {
    /// Una tecla física (`KeyboardEvent.code`).
    Key { code: String },
    /// Un botón del mouse.
    Mouse { button: MouseButton },
}

/// Dónde se puede activar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum EnhancerScope {
    /// Solo con un juego conocido (o agregado) al frente.
    #[default]
    Games,
    /// En cualquier programa.
    Always,
}

/// Color del tinte por defecto: un verde suave.
pub const DEFAULT_TINT: &str = "#7CFF6B";

/// `true` si es un color `#RRGGBB`.
fn is_hex_color(value: &str) -> bool {
    value.len() == 7 && value.starts_with('#') && value[1..].chars().all(|c| c.is_ascii_hexdigit())
}

/// El efecto de Dark Boost.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct DarkBoost {
    /// Cuánto se levantan las sombras (0–100).
    pub strength: f32,
    /// Cuánto se levantan los negros (0–100).
    pub black_level: f32,
    /// Teñir la imagen (por ejemplo, de verde para "visión nocturna").
    pub tint_enabled: bool,
    /// Color del tinte (`#RRGGBB`).
    pub tint: String,
    /// Intensidad del tinte (0–100).
    pub tint_amount: f32,
    /// Invierte los colores (puede molestar a personas fotosensibles).
    pub invert: bool,
}

impl Default for DarkBoost {
    fn default() -> Self {
        Self {
            strength: ranges::DARK_BOOST.default,
            black_level: ranges::BLACK_LEVEL.default,
            tint_enabled: false,
            tint: DEFAULT_TINT.to_owned(),
            tint_amount: ranges::TINT_AMOUNT.default,
            invert: false,
        }
    }
}

/// Ajustes de Dark Boost.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct DarkBoostSettings {
    /// Armado: la tecla funciona.
    pub armed: bool,
    pub mode: HoldMode,
    pub binding: Binding,
    pub scope: EnhancerScope,
    pub effect: DarkBoost,
}

impl Default for DarkBoostSettings {
    fn default() -> Self {
        Self {
            armed: false,
            mode: HoldMode::Hold,
            binding: Binding::Mouse {
                button: MouseButton::Forward,
            },
            scope: EnhancerScope::Games,
            effect: DarkBoost::default(),
        }
    }
}

/// Ajustes de Zoom.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct ZoomSettings {
    pub armed: bool,
    pub mode: HoldMode,
    pub binding: Binding,
    pub scope: EnhancerScope,
    /// Aumento: 1,5× a 4×.
    pub level: f32,
    /// Pantalla donde se hace zoom (`None`: la principal).
    pub display_id: Option<String>,
}

impl Default for ZoomSettings {
    fn default() -> Self {
        Self {
            armed: false,
            mode: HoldMode::Hold,
            binding: Binding::Mouse {
                button: MouseButton::Back,
            },
            scope: EnhancerScope::Games,
            level: ranges::ZOOM.default,
            display_id: None,
        }
    }
}

/// Dark Boost y Zoom.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct EnhancerSettings {
    pub dark_boost: DarkBoostSettings,
    pub zoom: ZoomSettings,
}

impl EnhancerSettings {
    /// El mismo con los valores dentro de sus rangos.
    pub fn clamped(&self) -> Self {
        let mut c = self.clone();
        c.dark_boost.effect.strength = ranges::DARK_BOOST.clamp(c.dark_boost.effect.strength);
        c.dark_boost.effect.black_level =
            ranges::BLACK_LEVEL.clamp(c.dark_boost.effect.black_level);
        c.dark_boost.effect.tint_amount =
            ranges::TINT_AMOUNT.clamp(c.dark_boost.effect.tint_amount);
        if !is_hex_color(&c.dark_boost.effect.tint) {
            c.dark_boost.effect.tint = DEFAULT_TINT.to_owned();
        }
        c.zoom.level = ranges::ZOOM.clamp(c.zoom.level);
        c
    }
}

/// Aumento imperceptible (menos de un tercio de píxel en 4800 px) para dejar
/// listo el zoom de Windows: la primera vez tarda (unos 330 ms en la PC de
/// pruebas) y así no se nota al apretar la tecla.
pub const ZOOM_WARMUP_LEVEL: f32 = 1.0001;

/// ¿Se escucha la tecla? Solo armada y, salvo con "siempre", con un juego al frente.
pub fn listens(armed: bool, scope: EnhancerScope, in_game: bool) -> bool {
    armed && (scope == EnhancerScope::Always || in_game)
}

/// Sigue una tecla de "mantener" o de "alternar" a partir de lecturas sueltas.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HoldTracker {
    active: bool,
    was_down: bool,
}

impl HoldTracker {
    /// Nueva lectura de la tecla. Devuelve `true` si cambió si está activo.
    pub fn update(&mut self, mode: HoldMode, down: bool) -> bool {
        let pressed = down && !self.was_down;
        self.was_down = down;
        let active = match mode {
            HoldMode::Hold => down,
            HoldMode::Toggle => self.active != pressed,
        };
        let changed = active != self.active;
        self.active = active;
        changed
    }

    /// Está activo (la tecla apretada, o alternado a prendido).
    pub fn active(&self) -> bool {
        self.active
    }

    /// Suelta todo (al salir del juego, al desarmar o con el atajo de
    /// emergencia). Devuelve `true` si estaba activo.
    pub fn release(&mut self) -> bool {
        std::mem::take(self).active
    }
}

/// Desplazamiento para `MagSetFullscreenTransform` que hace zoom `level`
/// manteniendo fijo el punto `center` (coordenadas del escritorio virtual,
/// relativas al monitor principal; pueden ser negativas).
pub fn zoom_offset(level: f32, center: (i32, i32)) -> (i32, i32) {
    let keep = 1.0 - 1.0 / level.max(1.0);
    (
        (center.0 as f32 * keep).round() as i32,
        (center.1 as f32 * keep).round() as i32,
    )
}

/// El tono con las sombras y los negros levantados por Dark Boost.
pub fn boosted_tone(tone: &Tone, boost: &DarkBoost) -> Tone {
    Tone {
        shadows: ranges::TONAL.clamp(tone.shadows + boost.strength),
        blacks: ranges::TONAL.clamp(tone.blacks + boost.black_level),
        ..tone.clone()
    }
}

/// La parte de matriz de Dark Boost: tinte (sin desplazamientos) e inversión
/// (con desplazamientos: no se usa con HDR).
pub fn dark_boost_matrix(boost: &DarkBoost, allow_offsets: bool) -> ColorMatrix {
    let mut matrix = ColorMatrix::IDENTITY;
    if boost.tint_enabled {
        let digits = boost.tint.trim().trim_start_matches('#');
        let channel = |i: usize| {
            digits
                .get(i..i + 2)
                .and_then(|pair| u8::from_str_radix(pair, 16).ok())
                .map_or(1.0, |value| f32::from(value) / 255.0)
        };
        let rgb = [channel(0), channel(2), channel(4)];
        let peak = rgb.iter().copied().fold(0.0f32, f32::max).max(1e-3);
        let mut tinted = ColorMatrix::IDENTITY;
        for (i, value) in rgb.iter().enumerate() {
            tinted.0[i][i] = value / peak;
        }
        matrix = matrix.lerp(&tinted, (boost.tint_amount / 100.0).clamp(0.0, 1.0));
    }
    if boost.invert && allow_offsets {
        matrix = matrix.then(&invert());
    }
    matrix
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_keeps_the_center_fixed() {
        for center in [(960, 540), (-960, 540), (1440, 900), (3000, -200)] {
            for level in [1.5f32, 2.0, 3.25, 4.0] {
                let (x, y) = zoom_offset(level, center);
                // Un punto P de la pantalla se ve en (P − desplazamiento) × aumento.
                let seen_x = (center.0 - x) as f32 * level;
                let seen_y = (center.1 - y) as f32 * level;
                assert!(
                    (seen_x - center.0 as f32).abs() <= level,
                    "{center:?} ×{level}"
                );
                assert!(
                    (seen_y - center.1 as f32).abs() <= level,
                    "{center:?} ×{level}"
                );
            }
        }
        assert_eq!(
            zoom_offset(1.0, (500, 500)),
            (0, 0),
            "sin zoom, sin desplazamiento"
        );
    }

    #[test]
    fn dark_boost_lifts_shadows_and_blacks_within_range() {
        let tone = Tone {
            shadows: 70.0,
            ..Tone::default()
        };
        let boosted = boosted_tone(&tone, &DarkBoost::default());
        assert_eq!(boosted.shadows, 100.0, "se recorta al rango");
        assert!(boosted.blacks > 0.0);
        assert_eq!(boosted.gamma, tone.gamma);
    }

    #[test]
    fn tint_keeps_its_brightest_channel_and_invert_needs_offsets() {
        let boost = DarkBoost {
            tint_enabled: true,
            tint: "#66FF66".into(),
            tint_amount: 100.0,
            ..DarkBoost::default()
        };
        let matrix = dark_boost_matrix(&boost, true);
        let out = matrix.apply([1.0, 1.0, 1.0]);
        assert!((out[1] - 1.0).abs() < 1e-5, "verde intacto");
        assert!(out[0] < 0.5 && out[2] < 0.5, "rojo y azul bajan");
        assert!(!matrix.has_offsets());

        let inverted = DarkBoost {
            invert: true,
            ..DarkBoost::default()
        };
        assert!(dark_boost_matrix(&inverted, true).has_offsets());
        assert!(
            dark_boost_matrix(&inverted, false).is_identity(1e-6),
            "con HDR no se invierte"
        );
    }

    #[test]
    fn hold_follows_the_key_and_toggle_flips_on_each_press() {
        let mut hold = HoldTracker::default();
        assert!(!hold.update(HoldMode::Hold, false));
        assert!(hold.update(HoldMode::Hold, true) && hold.active());
        assert!(
            !hold.update(HoldMode::Hold, true),
            "mantener no cambia nada"
        );
        assert!(hold.update(HoldMode::Hold, false) && !hold.active());

        let mut toggle = HoldTracker::default();
        assert!(toggle.update(HoldMode::Toggle, true) && toggle.active());
        assert!(
            !toggle.update(HoldMode::Toggle, true),
            "mantener no vuelve a alternar"
        );
        assert!(
            !toggle.update(HoldMode::Toggle, false) && toggle.active(),
            "soltar no apaga"
        );
        assert!(
            toggle.update(HoldMode::Toggle, true) && !toggle.active(),
            "otra pulsación apaga"
        );

        toggle.update(HoldMode::Toggle, false);
        toggle.update(HoldMode::Toggle, true);
        assert!(toggle.release(), "estaba activo");
        assert!(!toggle.active() && !toggle.release());
    }

    #[test]
    fn listens_only_when_armed_and_in_scope() {
        assert!(
            !listens(false, EnhancerScope::Always, true),
            "desarmado, nunca"
        );
        assert!(
            !listens(true, EnhancerScope::Games, false),
            "sin juego al frente"
        );
        assert!(listens(true, EnhancerScope::Games, true));
        assert!(listens(true, EnhancerScope::Always, false));
    }

    #[test]
    fn a_bad_tint_color_goes_back_to_the_default() {
        let mut settings = EnhancerSettings::default();
        settings.dark_boost.effect.tint = "verde".into();
        assert_eq!(settings.clamped().dark_boost.effect.tint, DEFAULT_TINT);
        settings.dark_boost.effect.tint = "#12aBcD".into();
        assert_eq!(settings.clamped().dark_boost.effect.tint, "#12aBcD");
        assert!(
            dark_boost_matrix(&DarkBoost::default(), true).is_identity(1e-6),
            "sin tinte ni inversión, nada"
        );
    }

    #[test]
    fn defaults_are_disarmed_and_use_side_buttons() {
        let settings = EnhancerSettings::default();
        assert!(
            !settings.dark_boost.armed && !settings.zoom.armed,
            "vienen apagados"
        );
        assert_eq!(settings.zoom.scope, EnhancerScope::Games);
        assert_eq!(MouseButton::Back.virtual_key(), 0x05);
        let parsed: EnhancerSettings = serde_json::from_str("{}").expect("válido");
        assert_eq!(parsed, settings);
    }
}
