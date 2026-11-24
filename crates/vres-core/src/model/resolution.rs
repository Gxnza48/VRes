//! Resoluciones: los modos de cada pantalla, los presets estirados que propone
//! VRes y el estado de un cambio pendiente de confirmar.
//!
//! Todo es puro: `vres-win` enumera los modos y acá solo se ordenan, se
//! filtran y se calculan los presets.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::status::ReasonCode;

/// Ancho y alto mínimos que VRes acepta.
pub const MIN_WIDTH: u32 = 640;
/// Alto mínimo que VRes acepta.
pub const MIN_HEIGHT: u32 = 480;

/// Un modo de pantalla: ancho × alto @ frecuencia (Hz enteros, como los
/// informa Windows).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
}

impl DisplayMode {
    /// Crea un modo.
    pub const fn new(width: u32, height: u32, refresh_hz: u32) -> Self {
        Self {
            width,
            height,
            refresh_hz,
        }
    }

    /// Mismo ancho y alto (sin mirar la frecuencia).
    pub fn same_size(&self, other: &Self) -> bool {
        self.width == other.width && self.height == other.height
    }

    /// Relación de aspecto (ancho / alto).
    pub fn aspect(&self) -> f64 {
        f64::from(self.width) / f64::from(self.height.max(1))
    }
}

/// Cómo se lleva la imagen a la pantalla cuando el modo no es el nativo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Scaling {
    /// Lo decide el driver o el monitor.
    #[default]
    Preferred,
    /// Sin escalar (el modo coincide con el del monitor).
    Identity,
    /// Centrada, con barras alrededor.
    Centered,
    /// Estirada a toda la pantalla.
    Stretched,
    /// Agrandada manteniendo la relación de aspecto (con barras).
    AspectRatio,
    /// Otro valor propio del driver.
    Custom,
}

/// Un modo pedido, con su escalado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModeRequest {
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
    pub scaling: Scaling,
}

impl ModeRequest {
    /// El modo, sin el escalado.
    pub fn mode(&self) -> DisplayMode {
        DisplayMode::new(self.width, self.height, self.refresh_hz)
    }
}

/// Familia de un preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PresetGroup {
    /// 4:3 (por ejemplo 1440 × 1080).
    Aspect4x3,
    /// 5:4 (por ejemplo 1350 × 1080).
    Aspect5x4,
    /// 16:10 (por ejemplo 1728 × 1080).
    Aspect16x10,
    /// Anchos intermedios a la altura nativa: estiran un poco menos que 4:3.
    Popular,
}

/// Un preset de resolución propuesto por VRes para una pantalla.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ResolutionPreset {
    pub width: u32,
    pub height: u32,
    pub group: PresetGroup,
    /// Mejor frecuencia disponible para ese tamaño (0 si no está disponible).
    pub refresh_hz: u32,
    /// El driver lo lista: se puede aplicar ya. Si no, hace falta una
    /// resolución personalizada (llega en la v1.0).
    pub available: bool,
    /// Cuánto se estira en horizontal respecto de la nativa (1,33 = 33 % más ancha).
    pub stretch: f32,
}

/// Modos y estado de una pantalla.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DisplayModes {
    /// Identificador estable de la pantalla.
    pub display_id: String,
    /// Modo actual.
    pub current: DisplayMode,
    /// Modo guardado en Windows: al que se vuelve.
    pub saved: DisplayMode,
    /// Resolución nativa del monitor (la que pide su EDID), si se conoce.
    pub native: Option<DisplayMode>,
    /// Escalado actual, según Windows.
    pub scaling: Scaling,
    /// Modos que admite, del más grande al más chico.
    pub modes: Vec<DisplayMode>,
    /// Presets estirados que propone VRes.
    pub presets: Vec<ResolutionPreset>,
    /// VRes tiene aplicado un modo temporal en esta pantalla.
    pub changed: bool,
}

/// Un cambio de resolución esperando confirmación.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PendingMode {
    /// Se usa para confirmar o revertir este cambio y no otro.
    pub token: String,
    pub display_id: String,
    pub mode: ModeRequest,
    /// Tiempo que queda antes de volver solo.
    #[ts(type = "number")]
    pub expires_in_ms: u64,
    /// Duración total de la cuenta regresiva, en segundos.
    pub seconds: u32,
    /// Se aplicó, pero con una limitación (por ejemplo, el driver no estiró).
    pub reason: Option<ReasonCode>,
}

/// Estado de la pestaña Resolución (evento `resolution:changed`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ResolutionState {
    /// Aumenta cada vez que cambia algo.
    #[ts(type = "number")]
    pub rev: u64,
    pub displays: Vec<DisplayModes>,
    pub pending: Option<PendingMode>,
}

/// Cómo terminó un cambio de resolución.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ModeOutcome {
    /// El usuario lo conservó.
    Kept,
    /// El usuario lo revirtió.
    Reverted,
    /// Nadie respondió a tiempo: volvió solo.
    Expired,
    /// No se pudo aplicar o revertir.
    Failed,
}

/// Resultado de un cambio de resolución (evento `resolution:resolved`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ModeResolved {
    pub token: String,
    pub display_id: String,
    pub mode: ModeRequest,
    pub outcome: ModeOutcome,
    pub reason: Option<ReasonCode>,
}

/// Lo agrega a la lista si cabe en la pantalla y no está repetido.
fn push_preset(
    presets: &mut Vec<ResolutionPreset>,
    native: &DisplayMode,
    modes: &[DisplayMode],
    width: u32,
    height: u32,
    group: PresetGroup,
) -> bool {
    if width < MIN_WIDTH
        || height < MIN_HEIGHT
        || width > native.width
        || height > native.height
        || (width == native.width && height == native.height)
        || presets
            .iter()
            .any(|preset| preset.width == width && preset.height == height)
    {
        return false;
    }
    let size = DisplayMode::new(width, height, 0);
    let rates: Vec<u32> = modes
        .iter()
        .filter(|mode| mode.same_size(&size))
        .map(|mode| mode.refresh_hz)
        .collect();
    // La misma frecuencia que la nativa si existe; si no, la más alta.
    let refresh_hz = if rates.contains(&native.refresh_hz) {
        native.refresh_hz
    } else {
        rates.iter().copied().max().unwrap_or(0)
    };
    presets.push(ResolutionPreset {
        width,
        height,
        group,
        refresh_hz,
        available: !rates.is_empty(),
        stretch: (native.aspect() / size.aspect()) as f32,
    });
    true
}

/// Proporciones que estiran respecto de la nativa (se saltean las casi
/// iguales a la de la pantalla: no estirarían nada).
fn stretched_groups(native: &DisplayMode) -> Vec<(PresetGroup, f64)> {
    [
        (PresetGroup::Aspect4x3, 4.0 / 3.0),
        (PresetGroup::Aspect5x4, 5.0 / 4.0),
        (PresetGroup::Aspect16x10, 16.0 / 10.0),
    ]
    .into_iter()
    .filter(|(_, ratio)| (native.aspect() / ratio - 1.0).abs() > 0.02)
    .collect()
}

fn same_ratio(width: u32, height: u32, ratio: f64) -> bool {
    (f64::from(width) / f64::from(height) / ratio - 1.0).abs() < 0.01
}

/// Ancho par más cercano para una altura y una proporción.
fn width_for(height: u32, ratio: f64) -> u32 {
    let width = (f64::from(height) * ratio / 2.0).round() as u32 * 2;
    width.max(2)
}

/// Presets estirados de VRes para una pantalla con esa resolución nativa.
///
/// Son propios: cada proporción a la altura nativa, anchos intermedios que
/// estiran menos que 4:3, los tamaños con esas proporciones que el monitor ya
/// lista y algunos clásicos más chicos. Cada uno indica si el driver ya lo
/// lista o si requiere una resolución personalizada.
pub fn presets(native: &DisplayMode, modes: &[DisplayMode]) -> Vec<ResolutionPreset> {
    let mut list = Vec::new();
    let height = native.height;
    let groups = stretched_groups(native);

    for &(group, ratio) in &groups {
        push_preset(
            &mut list,
            native,
            modes,
            width_for(height, ratio),
            height,
            group,
        );
    }

    // Intermedios: solo tienen sentido en pantallas más anchas que 16:10.
    if native.aspect() > 1.65 {
        for ratio in [
            1600.0 / 1080.0,
            1680.0 / 1080.0,
            1720.0 / 1080.0,
            1750.0 / 1080.0,
        ] {
            push_preset(
                &mut list,
                native,
                modes,
                width_for(height, ratio),
                height,
                PresetGroup::Popular,
            );
        }
    }

    // Lo que el monitor ya lista con esas proporciones: hasta 3 por familia,
    // de los más grandes (más del 60 % de la altura nativa).
    let mut sizes: Vec<(u32, u32)> = modes.iter().map(|mode| (mode.width, mode.height)).collect();
    sizes.sort_by_key(|&(width, height)| std::cmp::Reverse((height, width)));
    sizes.dedup();
    for &(group, ratio) in &groups {
        let mut added = 0;
        for &(width, size_height) in &sizes {
            if added == 3 {
                break;
            }
            if size_height * 10 >= height * 6
                && same_ratio(width, size_height, ratio)
                && push_preset(&mut list, native, modes, width, size_height, group)
            {
                added += 1;
            }
        }
    }

    // Clásicos más chicos (más FPS en equipos modestos).
    for (width, size_height) in [
        (1280, 960),
        (1024, 768),
        (1280, 1024),
        (1680, 1050),
        (1440, 900),
    ] {
        if let Some(&(group, _)) = groups
            .iter()
            .find(|(_, ratio)| same_ratio(width, size_height, *ratio))
        {
            push_preset(&mut list, native, modes, width, size_height, group);
        }
    }

    // Los que se pueden aplicar ya, primero; después, de mayor a menor.
    list.sort_by_key(|preset| {
        (
            !preset.available,
            preset.group as u8,
            std::cmp::Reverse(preset.height),
            std::cmp::Reverse(preset.width),
        )
    });
    list
}

/// Ordena de mayor a menor (tamaño y frecuencia) y quita los repetidos.
pub fn normalize_modes(mut modes: Vec<DisplayMode>) -> Vec<DisplayMode> {
    modes.retain(|mode| mode.width >= MIN_WIDTH && mode.height >= MIN_HEIGHT);
    modes.sort_by_key(|mode| std::cmp::Reverse((mode.width, mode.height, mode.refresh_hz)));
    modes.dedup();
    modes
}

/// `true` si el driver lista ese modo exacto (tamaño y frecuencia).
pub fn is_listed(modes: &[DisplayMode], request: &ModeRequest) -> bool {
    modes.contains(&request.mode())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NATIVE_1080: DisplayMode = DisplayMode::new(1920, 1080, 200);

    fn modes_1080() -> Vec<DisplayMode> {
        normalize_modes(vec![
            DisplayMode::new(1920, 1080, 200),
            DisplayMode::new(1920, 1080, 144),
            DisplayMode::new(1920, 1080, 60),
            DisplayMode::new(1440, 1080, 200),
            DisplayMode::new(1440, 1080, 60),
            DisplayMode::new(1280, 1024, 75),
            DisplayMode::new(1280, 960, 60),
            DisplayMode::new(1024, 768, 60),
            DisplayMode::new(800, 600, 60),
            DisplayMode::new(640, 480, 60),
            DisplayMode::new(1280, 720, 60),
            DisplayMode::new(1280, 720, 60),
        ])
    }

    #[test]
    fn modes_are_sorted_and_unique() {
        let modes = modes_1080();
        assert_eq!(modes[0], DisplayMode::new(1920, 1080, 200));
        assert_eq!(modes[1], DisplayMode::new(1920, 1080, 144));
        let mut unique = modes.clone();
        unique.dedup();
        assert_eq!(unique, modes);
        assert!(modes.iter().all(|mode| mode.width >= MIN_WIDTH));
    }

    #[test]
    fn classic_stretched_presets_at_native_height() {
        let list = presets(&NATIVE_1080, &modes_1080());
        let find = |w, h| list.iter().find(|p| p.width == w && p.height == h);

        let four_three = find(1440, 1080).expect("4:3");
        assert!(four_three.available);
        assert_eq!(four_three.refresh_hz, 200, "prefiere la frecuencia nativa");
        assert!((four_three.stretch - 4.0 / 3.0).abs() < 0.01);

        let five_four = find(1350, 1080).expect("5:4");
        assert!(!five_four.available, "1350 × 1080 no está en la lista");
        assert_eq!(five_four.refresh_hz, 0);

        assert!(find(1728, 1080).is_some(), "16:10");
        for width in [1600, 1680, 1720, 1750] {
            let preset = find(width, 1080).expect("intermedio");
            assert_eq!(preset.group, PresetGroup::Popular);
        }
        assert_eq!(find(1280, 1024).map(|p| p.refresh_hz), Some(75));
    }

    #[test]
    fn presets_never_exceed_or_repeat_the_native_mode() {
        let list = presets(&NATIVE_1080, &modes_1080());
        assert!(list.iter().all(|p| p.width <= 1920 && p.height <= 1080));
        assert!(!list.iter().any(|p| p.width == 1920 && p.height == 1080));
        let mut sizes: Vec<_> = list.iter().map(|p| (p.width, p.height)).collect();
        sizes.sort_unstable();
        sizes.dedup();
        assert_eq!(sizes.len(), list.len());
    }

    #[test]
    fn available_presets_come_first() {
        let list = presets(&NATIVE_1080, &modes_1080());
        let first_missing = list
            .iter()
            .position(|p| !p.available)
            .expect("hay faltantes");
        assert!(list[..first_missing].iter().all(|p| p.available));
        assert!(list[first_missing..].iter().all(|p| !p.available));
    }

    #[test]
    fn sixteen_ten_panels_skip_the_intermediate_widths() {
        let native = DisplayMode::new(2880, 1800, 120);
        let list = presets(&native, &[]);
        assert!(list.iter().all(|p| p.group != PresetGroup::Popular));
        assert!(
            list.iter().any(|p| p.width == 2400 && p.height == 1800),
            "4:3"
        );
        assert!(
            list.iter().any(|p| p.width == 2250 && p.height == 1800),
            "5:4"
        );
        assert!(!list.iter().any(|p| p.width == 2880), "16:10 es la nativa");
    }

    #[test]
    fn listed_stretched_sizes_at_other_heights_are_proposed() {
        let native = DisplayMode::new(2880, 1800, 120);
        let modes = normalize_modes(vec![
            DisplayMode::new(2880, 1800, 120),
            DisplayMode::new(2048, 1536, 120),
            DisplayMode::new(1600, 1200, 120),
            DisplayMode::new(1280, 1024, 120),
            DisplayMode::new(1680, 1050, 120),
            DisplayMode::new(800, 600, 60),
        ]);
        let list = presets(&native, &modes);
        let find = |w, h| list.iter().find(|p| p.width == w && p.height == h);
        assert!(find(2048, 1536).is_some_and(|p| p.available && p.group == PresetGroup::Aspect4x3));
        assert!(find(1600, 1200).is_some_and(|p| p.available));
        assert!(find(1280, 1024).is_some_and(|p| p.available && p.group == PresetGroup::Aspect5x4));
        assert!(
            find(2400, 1800).is_some_and(|p| !p.available),
            "4:3 a la altura nativa"
        );
        assert!(
            find(1680, 1050).is_none(),
            "misma proporción que la nativa: no estira"
        );
        assert!(find(800, 600).is_none(), "demasiado chica");
        assert_eq!(list[0].width, 2048, "primero lo disponible y más grande");
    }

    #[test]
    fn listed_means_exact_size_and_rate() {
        let modes = modes_1080();
        let request = ModeRequest {
            width: 1440,
            height: 1080,
            refresh_hz: 200,
            scaling: Scaling::Stretched,
        };
        assert!(is_listed(&modes, &request));
        assert!(!is_listed(
            &modes,
            &ModeRequest {
                refresh_hz: 144,
                ..request
            }
        ));
    }

    #[test]
    fn requests_serialize_in_camel_case() {
        let value = serde_json::to_value(ModeRequest {
            width: 1440,
            height: 1080,
            refresh_hz: 200,
            scaling: Scaling::AspectRatio,
        })
        .expect("serializable");
        assert_eq!(value["refreshHz"], 200);
        assert_eq!(value["scaling"], "aspectRatio");
    }
}
