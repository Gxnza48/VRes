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
    fn lod_bias_is_in_eighths() {
        let potato = values(PotatoLevel::Potato);
        assert_eq!(potato[0], (LOD_BIAS_DX, Some(15 * 8)));
        assert_eq!(values(PotatoLevel::Low)[0], (LOD_BIAS_DX, Some(3 * 8)));
    }
}
