//! Ajustes del juego para más FPS: cambia opciones del archivo de ajustes que
//! escribe el propio menú de gráficos del juego, con el juego cerrado.
//!
//! Solo se usan valores que ese menú ofrece. VRes recuerda lo que tenía el
//! jugador para devolverlo con «Restaurar mis ajustes».

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Un ajuste del preset: clave del archivo y el valor JSON a escribir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TuneSetting {
    pub key: &'static str,
    pub value: &'static str,
}

const fn set(key: &'static str, value: &'static str) -> TuneSetting {
    TuneSetting { key, value }
}

/// Preset «máximo FPS» de ROB RUN. No toca resolución, escala de render,
/// límite de FPS ni texturas (eso queda para el jugador y para el Potato).
pub const ROBRUN_MAX_FPS: [TuneSetting; 13] = [
    // «Personalizado»: si quedara un preset del juego, pisaría estos valores.
    set("setting_quality", "\"custom\""),
    set("anti_aliasing_mode_id_setting", "\"disabled\""),
    set("cast_shadow_setting", "\"disabled\""),
    set("additional_cast_lights_setting", "false"),
    set("hbao_setting", "false"),
    set("screen_space_reflections_setting", "false"),
    set("depth_of_field", "false"),
    set("motion_blur_setting", "false"),
    set("settings_grass", "\"low\""),
    set("grass_vibile_distance_factor_setting", "\"low\""),
    set("settings_water", "\"minimum\""),
    set("renderers_culling_preset_id_setting", "\"low\""),
    // Contador de FPS del juego, para ver la diferencia.
    set("performance_info_fps", "true"),
];

/// Juegos con preset de FPS.
pub fn preset(game_id: &str) -> Option<&'static [TuneSetting]> {
    match game_id {
        "robrun" => Some(&ROBRUN_MAX_FPS),
        _ => None,
    }
}

/// Un ajuste que el preset cambiaría (o ya cambió).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameTuneChange {
    /// Clave del archivo del juego (la interfaz la traduce).
    pub key: String,
    /// Valor actual, como texto JSON (`false`, `"low"`…).
    pub current: String,
    /// Valor del preset.
    pub target: String,
}

/// Estado del preset de FPS de un juego.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GameTuneStatus {
    pub game_id: String,
    pub game_name: String,
    /// Hay archivo de ajustes (el juego se abrió al menos una vez).
    pub found: bool,
    /// El juego está abierto: hay que cerrarlo para cambiar algo.
    pub game_running: bool,
    /// VRes tiene guardados los valores anteriores del jugador.
    pub applied: bool,
    /// Ajustes del preset con su valor actual.
    pub settings: Vec<GameTuneChange>,
}

/// Dónde está el valor de una clave en un JSON plano (sin espacios, como lo
/// escribe el juego): de la primera letra del valor al final del valor.
fn value_span(text: &str, key: &str) -> Option<(usize, usize)> {
    let pattern = format!("\"{key}\":");
    let start = text.find(&pattern)? + pattern.len();
    let rest = &text[start..];
    let len = if rest.starts_with('"') {
        let mut escaped = false;
        let mut end = None;
        for (index, byte) in rest.bytes().enumerate().skip(1) {
            match byte {
                b'\\' if !escaped => escaped = true,
                b'"' if !escaped => {
                    end = Some(index + 1);
                    break;
                }
                _ => escaped = false,
            }
        }
        end?
    } else {
        rest.find([',', '}', ']']).unwrap_or(rest.len())
    };
    // Solo valores simples: un objeto o una lista no se tocan.
    let value = rest[..len].trim();
    if value.is_empty() || value.starts_with('{') || value.starts_with('[') {
        return None;
    }
    Some((start, start + len))
}

/// Lee el valor de una clave como texto JSON.
pub fn read_value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    value_span(text, key).map(|(start, end)| &text[start..end])
}

/// Cambia el valor de una clave y deja el resto del archivo igual.
pub fn write_value(text: &str, key: &str, value: &str) -> Option<String> {
    let (start, end) = value_span(text, key)?;
    Some(format!("{}{value}{}", &text[..start], &text[end..]))
}

/// Ajustes del preset con su valor actual (los que el archivo no tiene se omiten).
pub fn settings(text: &str, preset: &[TuneSetting]) -> Vec<GameTuneChange> {
    preset
        .iter()
        .filter_map(|setting| {
            read_value(text, setting.key).map(|current| GameTuneChange {
                key: setting.key.to_string(),
                current: current.to_string(),
                target: setting.value.to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"setting_quality":"high","sound_volume":0.600000024,"hbao_setting":true,"settings_grass":"high","controls":"{\"bindings\":[]}","motion_blur_setting":true}"#;

    #[test]
    fn reads_every_kind_of_value() {
        assert_eq!(read_value(SAMPLE, "setting_quality"), Some("\"high\""));
        assert_eq!(read_value(SAMPLE, "sound_volume"), Some("0.600000024"));
        assert_eq!(read_value(SAMPLE, "hbao_setting"), Some("true"));
        assert_eq!(read_value(SAMPLE, "motion_blur_setting"), Some("true"));
        assert_eq!(
            read_value(SAMPLE, "controls"),
            Some(r#""{\"bindings\":[]}""#)
        );
        assert_eq!(read_value(SAMPLE, "missing"), None);
    }

    #[test]
    fn writes_only_that_value() {
        let edited = write_value(SAMPLE, "settings_grass", "\"low\"").unwrap();
        assert_eq!(
            edited,
            SAMPLE.replace("\"settings_grass\":\"high\"", "\"settings_grass\":\"low\"")
        );
        let edited = write_value(&edited, "hbao_setting", "false").unwrap();
        assert!(edited.contains("\"hbao_setting\":false,"));
        assert!(edited.contains("0.600000024"));
        assert!(edited.contains(r#""controls":"{\"bindings\":[]}""#));
    }

    #[test]
    fn lists_only_settings_the_file_has() {
        let list = settings(SAMPLE, &ROBRUN_MAX_FPS);
        let keys: Vec<&str> = list.iter().map(|change| change.key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "setting_quality",
                "hbao_setting",
                "motion_blur_setting",
                "settings_grass"
            ]
        );
        assert_eq!(list[1].current, "true");
        assert_eq!(list[1].target, "false");
    }

    #[test]
    fn preset_values_are_plain_json() {
        for setting in ROBRUN_MAX_FPS {
            let parsed: serde_json::Value = serde_json::from_str(setting.value).unwrap();
            assert!(parsed.is_string() || parsed.is_boolean(), "{}", setting.key);
        }
    }
}
