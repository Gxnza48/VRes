//! Gráficos "potato" para placas NVIDIA: texturas planas y sin detalle a
//! cambio de FPS, con los ajustes del perfil del juego en el driver.
//!
//! Se escriben con la misma API que usa el panel de NVIDIA (perfiles DRS):
//! VRes no toca el juego. Los valores salen de la documentación pública de
//! NVAPI (`NvApiDriverSettings.h`, MIT).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Texture filtering - LOD Bias (DX). Entero con signo en octavos: +3,0 = 24.
pub const LOD_BIAS_DX: u32 = 0x0073_8E8F;
/// Texture filtering - LOD Bias (OpenGL), misma escala.
pub const LOD_BIAS_GL: u32 = 0x2040_3F79;
/// LOD Bias controlado por el driver: 0 = apagado (respeta el valor manual).
pub const LOD_BIAS_AUTO: u32 = 0x0063_8E8F;
/// Negative LOD bias: 0 = permitir, 1 = limitar.
pub const NEGATIVE_LOD: u32 = 0x0019_BB68;
/// Texture filtering - Quality: 0x14 = alto rendimiento.
pub const TEXTURE_QUALITY: u32 = 0x00CE_2691;
/// Antialiasing - Transparency Supersampling: 8 = todo (hace falta para que
/// algunos juegos respeten el LOD Bias).
pub const TRANSPARENCY_AA: u32 = 0x10D4_8A85;

/// Todos los ajustes que puede tocar VRes (para leerlos y para quitarlos).
pub const ALL_SETTINGS: [u32; 6] = [
    LOD_BIAS_DX,
    LOD_BIAS_GL,
    LOD_BIAS_AUTO,
    NEGATIVE_LOD,
    TEXTURE_QUALITY,
    TRANSPARENCY_AA,
];

/// Nombre del perfil que crea VRes si el juego no tiene uno en el driver.
pub const PROFILE_NAME: &str = "VRes - Potato";

/// Cuánto se aplanan las texturas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PotatoLevel {
    /// Sin cambios: vale lo del driver.
    #[default]
    Off,
    /// Texturas bajas (LOD +3): más FPS y menos ruido visual.
    Low,
    /// Potato (LOD +15): texturas planas, como de arcilla.
    Potato,
}

/// Un juego que admite gráficos potato.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PotatoGame {
    /// Id del juego (el mismo de los perfiles).
    pub id: String,
    pub name: String,
    /// Ejecutable que dibuja (no el lanzador del anti-cheat).
    pub executable: String,
}

/// Juegos con potato de fábrica (DirectX 11: el driver respeta el LOD Bias).
pub fn games() -> Vec<PotatoGame> {
    vec![PotatoGame {
        id: "robrun".into(),
        name: "ROB RUN".into(),
        executable: "ROB RUN.exe".into(),
    }]
}

/// Estado del potato de un juego.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PotatoStatus {
    /// Hay driver de NVIDIA con perfiles.
    pub available: bool,
    pub game: PotatoGame,
    /// Lo que está puesto ahora en el driver.
    pub level: PotatoLevel,
    /// El perfil tiene ajustes de LOD que no son de VRes.
    pub custom: bool,
    /// Driver viejo (rama anterior a 500, placas como la GT 730) que puede
    /// ignorar el LOD Bias en DirectX 11: VRes además baja el límite de
    /// texturas en el archivo de ajustes del juego.
    pub legacy: bool,
}

/// Primera rama del driver de NVIDIA en la que VRes confía para el LOD Bias
/// en DirectX 11 (las anteriores son de placas Kepler y más viejas).
pub const MODERN_DRIVER: u32 = 50_000;

/// `true` si el driver (versión en centésimas: 475.14 = 47514) es de una rama
/// anterior a la 500.
pub fn legacy_driver(version: u32) -> bool {
    version > 0 && version < MODERN_DRIVER
}

/// Clave del límite de texturas de Unity en los ajustes de ROB RUN.
pub const MIPMAP_KEY: &str = "global_texture_mipmap_limit";

/// Límite de texturas para un nivel (`None` = volver al del jugador).
/// Cada paso divide la textura a la mitad; 15 deja solo el color promedio.
pub fn mipmap_limit(level: PotatoLevel) -> Option<u32> {
    match level {
        PotatoLevel::Off => None,
        PotatoLevel::Low => Some(3),
        PotatoLevel::Potato => Some(15),
    }
}

/// Posición del número de `"clave":` en un JSON plano (sin parsearlo, para no
/// reescribir el resto del archivo del juego).
fn number_span(text: &str, key: &str) -> Option<(usize, usize)> {
    let pattern = format!("\"{key}\":");
    let start = text.find(&pattern)? + pattern.len();
    let digits = text[start..]
        .bytes()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    (digits > 0).then_some((start, start + digits))
}

/// Lee un entero de un JSON plano.
pub fn read_int(text: &str, key: &str) -> Option<u32> {
    let (start, end) = number_span(text, key)?;
    text[start..end].parse().ok()
}

/// Cambia un entero de un JSON plano y deja todo lo demás igual.
pub fn write_int(text: &str, key: &str, value: u32) -> Option<String> {
    let (start, end) = number_span(text, key)?;
    Some(format!("{}{value}{}", &text[..start], &text[end..]))
}

/// Valores a escribir para un nivel (`None` = quitar el ajuste del perfil).
pub fn values(level: PotatoLevel) -> Vec<(u32, Option<u32>)> {
    let lod = match level {
        PotatoLevel::Off => None,
        PotatoLevel::Low => Some(24),
        PotatoLevel::Potato => Some(120),
    };
    let on = |value: u32| lod.map(|_| value);
    vec![
        (LOD_BIAS_DX, lod),
        (LOD_BIAS_GL, lod),
        (LOD_BIAS_AUTO, on(0)),
        (NEGATIVE_LOD, on(0)),
        (TEXTURE_QUALITY, on(0x14)),
        (TRANSPARENCY_AA, (level == PotatoLevel::Potato).then_some(8)),
    ]
}

/// Qué nivel está puesto, a partir de los valores leídos del perfil.
/// Devuelve también si hay un LOD Bias propio que no coincide con VRes.
pub fn detect(read: &[(u32, Option<u32>)]) -> (PotatoLevel, bool) {
    let value = |id: u32| {
        read.iter()
            .find(|(key, _)| *key == id)
            .and_then(|(_, v)| *v)
    };
    match value(LOD_BIAS_DX) {
        None => (PotatoLevel::Off, false),
        Some(120) => (PotatoLevel::Potato, false),
        Some(24) => (PotatoLevel::Low, false),
        Some(_) => (PotatoLevel::Off, true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_removes_every_setting() {
        assert!(
            values(PotatoLevel::Off)
                .iter()
                .all(|(_, value)| value.is_none())
        );
        let ids: Vec<u32> = values(PotatoLevel::Off).iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, ALL_SETTINGS);
    }

    #[test]
    fn levels_round_trip_through_detect() {
        for level in [PotatoLevel::Off, PotatoLevel::Low, PotatoLevel::Potato] {
            assert_eq!(detect(&values(level)), (level, false));
        }
        assert_eq!(detect(&[(LOD_BIAS_DX, Some(8))]), (PotatoLevel::Off, true));
    }

    #[test]
    fn legacy_is_before_branch_500() {
        assert!(legacy_driver(47514));
        assert!(!legacy_driver(50000));
        assert!(!legacy_driver(58102));
        assert!(!legacy_driver(0));
    }

    #[test]
    fn edits_only_the_mipmap_number() {
        let text = r#"{"anisotropic_filtering":2,"global_texture_mipmap_limit":2,"fullscreen_setting":true}"#;
        assert_eq!(read_int(text, MIPMAP_KEY), Some(2));
        let edited = write_int(text, MIPMAP_KEY, 15).unwrap();
        assert_eq!(
            edited,
            r#"{"anisotropic_filtering":2,"global_texture_mipmap_limit":15,"fullscreen_setting":true}"#
        );
        assert_eq!(read_int(&edited, MIPMAP_KEY), Some(15));
        assert_eq!(write_int("{}", MIPMAP_KEY, 1), None);
        assert_eq!(
            read_int(r#"{"global_texture_mipmap_limit":"x"}"#, MIPMAP_KEY),
            None
        );
    }

    #[test]
    fn lod_bias_is_in_eighths() {
        let potato = values(PotatoLevel::Potato);
        assert_eq!(potato[0], (LOD_BIAS_DX, Some(15 * 8)));
        assert_eq!(values(PotatoLevel::Low)[0], (LOD_BIAS_DX, Some(3 * 8)));
    }
}
