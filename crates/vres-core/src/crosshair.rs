//! Crosshair: modelo, presets propios y el **renderer único**.
//!
//! La misma función [`render`] dibuja la vista previa de la interfaz y la
//! ventana transparente que se ve sobre el juego: lo que ves es lo que hay.
//! Sin rotación, los bordes caen justo en los píxeles (líneas nítidas); el
//! contorno se dibuja primero y el relleno lo reemplaza encima, así un relleno
//! semitransparente nunca se mezcla con su propio contorno.

use serde::{Deserialize, Serialize};
use tiny_skia::{
    BlendMode, Color, FillRule, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Stroke, Transform,
};
use ts_rs::TS;

use crate::model::ranges;

/// Lado máximo de un crosshair de imagen, en píxeles.
pub const MAX_IMAGE_SIDE: u32 = 512;

/// Vectorial (por partes) o una imagen propia.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CrosshairKind {
    #[default]
    Vector,
    Image,
}

/// Contorno de todas las partes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct Outline {
    pub enabled: bool,
    /// `#RRGGBB`.
    pub color: String,
    pub thickness: f32,
    pub opacity: f32,
}

impl Default for Outline {
    fn default() -> Self {
        Self {
            enabled: true,
            color: "#000000".into(),
            thickness: ranges::CROSSHAIR_OUTLINE.default,
            opacity: 100.0,
        }
    }
}

/// Los cuatro brazos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct Arms {
    pub enabled: bool,
    pub length: f32,
    pub thickness: f32,
    pub gap: f32,
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
    pub left: bool,
}

impl Default for Arms {
    fn default() -> Self {
        Self {
            enabled: true,
            length: ranges::CROSSHAIR_LENGTH.default,
            thickness: ranges::CROSSHAIR_THICKNESS.default,
            gap: ranges::CROSSHAIR_GAP.default,
            top: true,
            right: true,
            bottom: true,
            left: true,
        }
    }
}

/// El punto central.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct Dot {
    pub enabled: bool,
    pub size: f32,
    pub round: bool,
}

impl Default for Dot {
    fn default() -> Self {
        Self {
            enabled: false,
            size: ranges::CROSSHAIR_DOT.default,
            round: false,
        }
    }
}

/// Un anillo alrededor del centro.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct Ring {
    pub enabled: bool,
    pub radius: f32,
    pub thickness: f32,
}

impl Default for Ring {
    fn default() -> Self {
        Self {
            enabled: false,
            radius: ranges::CROSSHAIR_RADIUS.default,
            thickness: ranges::CROSSHAIR_RING.default,
        }
    }
}

/// Un crosshair.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct Crosshair {
    pub kind: CrosshairKind,
    /// `#RRGGBB`.
    pub color: String,
    /// 0–100 %.
    pub opacity: f32,
    pub outline: Outline,
    pub arms: Arms,
    pub dot: Dot,
    pub ring: Ring,
    /// Grados.
    pub rotation: f32,
    /// %.
    pub scale: f32,
    /// Desplazamiento desde el centro de la pantalla, en píxeles.
    pub offset_x: i32,
    pub offset_y: i32,
    /// Imagen propia: PNG como `data:image/png;base64,…` (hasta 512 × 512).
    pub image: Option<String>,
}

impl Default for Crosshair {
    fn default() -> Self {
        Self {
            kind: CrosshairKind::Vector,
            color: "#9EFA12".into(),
            opacity: 100.0,
            outline: Outline::default(),
            arms: Arms::default(),
            dot: Dot::default(),
            ring: Ring::default(),
            rotation: 0.0,
            scale: 100.0,
            offset_x: 0,
            offset_y: 0,
            image: None,
        }
    }
}

impl Crosshair {
    /// El mismo crosshair con todos los valores dentro de sus rangos.
    pub fn clamped(&self) -> Self {
        let mut c = self.clone();
        c.opacity = ranges::CROSSHAIR_OPACITY.clamp(c.opacity);
        c.outline.thickness = ranges::CROSSHAIR_OUTLINE.clamp(c.outline.thickness);
        c.outline.opacity = ranges::CROSSHAIR_OPACITY.clamp(c.outline.opacity);
        c.arms.length = ranges::CROSSHAIR_LENGTH.clamp(c.arms.length);
        c.arms.thickness = ranges::CROSSHAIR_THICKNESS.clamp(c.arms.thickness);
        c.arms.gap = ranges::CROSSHAIR_GAP.clamp(c.arms.gap);
        c.dot.size = ranges::CROSSHAIR_DOT.clamp(c.dot.size);
        c.ring.radius = ranges::CROSSHAIR_RADIUS.clamp(c.ring.radius);
        c.ring.thickness = ranges::CROSSHAIR_RING.clamp(c.ring.thickness);
        c.rotation = ranges::CROSSHAIR_ROTATION.clamp(c.rotation);
        c.scale = ranges::CROSSHAIR_SCALE.clamp(c.scale);
        let offset = ranges::CROSSHAIR_OFFSET;
        c.offset_x = (c.offset_x as f32).clamp(offset.min, offset.max) as i32;
        c.offset_y = (c.offset_y as f32).clamp(offset.min, offset.max) as i32;
        c
    }
}

/// Un crosshair incluido en VRes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CrosshairPreset {
    /// Identificador estable (la interfaz traduce el nombre).
    pub id: String,
    pub crosshair: Crosshair,
}

/// Presets propios de VRes.
pub fn presets() -> Vec<CrosshairPreset> {
    let base = Crosshair::default();
    let preset = |id: &str, crosshair: Crosshair| CrosshairPreset {
        id: id.to_owned(),
        crosshair,
    };
    vec![
        preset("classic", base.clone()),
        preset(
            "dot",
            Crosshair {
                arms: Arms {
                    enabled: false,
                    ..Arms::default()
                },
                dot: Dot {
                    enabled: true,
                    size: 4.0,
                    round: true,
                },
                ..base.clone()
            },
        ),
        preset(
            "thin",
            Crosshair {
                arms: Arms {
                    length: 8.0,
                    thickness: 1.0,
                    gap: 4.0,
                    ..Arms::default()
                },
                outline: Outline {
                    enabled: false,
                    ..Outline::default()
                },
                ..base.clone()
            },
        ),
        preset(
            "tshape",
            Crosshair {
                arms: Arms {
                    top: false,
                    ..Arms::default()
                },
                ..base.clone()
            },
        ),
        preset(
            "circle",
            Crosshair {
                arms: Arms {
                    enabled: false,
                    ..Arms::default()
                },
                dot: Dot {
                    enabled: true,
                    size: 2.0,
                    round: false,
                },
                ring: Ring {
                    enabled: true,
                    radius: 12.0,
                    thickness: 1.0,
                },
                ..base.clone()
            },
        ),
        preset(
            "plus",
            Crosshair {
                arms: Arms {
                    length: 9.0,
                    thickness: 2.0,
                    gap: 0.0,
                    ..Arms::default()
                },
                ..base.clone()
            },
        ),
        preset(
            "cross",
            Crosshair {
                rotation: 45.0,
                ..base.clone()
            },
        ),
        preset(
            "large",
            Crosshair {
                arms: Arms {
                    length: 18.0,
                    thickness: 3.0,
                    gap: 6.0,
                    ..Arms::default()
                },
                dot: Dot {
                    enabled: true,
                    size: 3.0,
                    round: false,
                },
                ..base
            },
        ),
    ]
}

/// Errores al dibujar.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CrosshairError {
    #[error("the image is not a PNG data URL")]
    NotPng,
    #[error("the image could not be decoded")]
    BadImage,
    #[error("the image is larger than {MAX_IMAGE_SIDE} × {MAX_IMAGE_SIDE}")]
    ImageTooLarge,
    #[error("nothing to draw")]
    Empty,
}

/// Imagen ya dibujada, con el centro del crosshair en `(center, center)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    /// Lado (siempre impar: hay un píxel central).
    pub size: u32,
    /// Índice del píxel central en cada eje.
    pub center: u32,
    /// RGBA con alfa premultiplicado, fila por fila.
    pub rgba: Vec<u8>,
}

impl Rendered {
    /// Píxel `(r, g, b, a)` premultiplicado.
    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.size + x) * 4) as usize;
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }

    /// RGBA sin premultiplicar (lo que espera un `<canvas>`).
    pub fn to_straight_rgba(&self) -> Vec<u8> {
        let mut out = self.rgba.clone();
        for px in out.as_chunks_mut::<4>().0 {
            let alpha = px[3];
            if alpha != 0 && alpha != 255 {
                for channel in &mut px[..3] {
                    *channel = ((u16::from(*channel) * 255 + u16::from(alpha) / 2)
                        / u16::from(alpha))
                    .min(255) as u8;
                }
            }
        }
        out
    }

    /// BGRA premultiplicado (lo que espera Windows en una ventana *layered*).
    pub fn to_bgra_premultiplied(&self) -> Vec<u8> {
        let mut out = self.rgba.clone();
        for px in out.as_chunks_mut::<4>().0 {
            px.swap(0, 2);
        }
        out
    }
}

fn parse_color(hex: &str, opacity: f32) -> Color {
    let digits = hex.trim().trim_start_matches('#');
    let channel = |i: usize| {
        digits
            .get(i..i + 2)
            .and_then(|pair| u8::from_str_radix(pair, 16).ok())
            .unwrap_or(255)
    };
    let (r, g, b) = if digits.len() == 6 {
        (channel(0), channel(2), channel(4))
    } else {
        (255, 255, 255)
    };
    Color::from_rgba8(r, g, b, (opacity.clamp(0.0, 100.0) * 2.55).round() as u8)
}

/// Decodifica Base64 estándar (sin dependencias).
fn decode_base64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut buffer, mut bits) = (0u32, 0u32);
    for byte in text.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' | b'\r' | b'\n' | b' ' => continue,
            _ => return None,
        };
        buffer = (buffer << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    Some(out)
}

/// Decodifica la imagen propia de un crosshair.
pub fn decode_image(data_url: &str) -> Result<Pixmap, CrosshairError> {
    let payload = data_url
        .strip_prefix("data:image/png;base64,")
        .ok_or(CrosshairError::NotPng)?;
    let bytes = decode_base64(payload).ok_or(CrosshairError::NotPng)?;
    let image = Pixmap::decode_png(&bytes).map_err(|_| CrosshairError::BadImage)?;
    if image.width() > MAX_IMAGE_SIDE || image.height() > MAX_IMAGE_SIDE {
        return Err(CrosshairError::ImageTooLarge);
    }
    Ok(image)
}

/// Extremo de un tramo de grosor `t` (en píxeles) centrado en el píxel
/// central: con grosor impar queda centrado; con par, un píxel más abajo.
fn span(t: f32) -> (f32, f32) {
    let t = t.round().max(1.0);
    if t as u32 % 2 == 1 {
        (-t / 2.0, t / 2.0)
    } else {
        (-t / 2.0 + 0.5, t / 2.0 + 0.5)
    }
}

/// Las formas de un crosshair vectorial, en coordenadas relativas al centro
/// del píxel central.
struct Shapes {
    rects: Vec<Rect>,
    circle_dot: Option<(f32, f32, f32)>,
    ring: Option<(f32, f32)>,
}

fn shapes(c: &Crosshair, s: f32) -> Shapes {
    let px = |value: f32| (value * s).round();
    let mut rects = Vec::new();
    if c.arms.enabled && px(c.arms.length) > 0.0 {
        let (a, b) = span(px(c.arms.thickness));
        let near = 0.5 + px(c.arms.gap);
        let far = near + px(c.arms.length);
        let mut push = |left: f32, top: f32, right: f32, bottom: f32| {
            if let Some(rect) = Rect::from_ltrb(left, top, right, bottom) {
                rects.push(rect);
            }
        };
        if c.arms.right {
            push(near, a, far, b);
        }
        if c.arms.left {
            push(-far, a, -near, b);
        }
        if c.arms.top {
            push(a, -far, b, -near);
        }
        if c.arms.bottom {
            push(a, near, b, far);
        }
    }
    let mut circle_dot = None;
    if c.dot.enabled {
        let size = px(c.dot.size).max(1.0);
        let (a, b) = span(size);
        if c.dot.round {
            let center = (a + b) / 2.0;
            circle_dot = Some((center, center, size / 2.0));
        } else if let Some(rect) = Rect::from_ltrb(a, a, b, b) {
            rects.push(rect);
        }
    }
    let ring = c
        .ring
        .enabled
        .then(|| (px(c.ring.radius).max(1.0), px(c.ring.thickness).max(1.0)));
    Shapes {
        rects,
        circle_dot,
        ring,
    }
}

/// Distancia máxima desde el centro que ocupa el dibujo.
fn extent(c: &Crosshair, shapes: &Shapes, outline: f32) -> f32 {
    let mut radius: f32 = 1.0;
    for rect in &shapes.rects {
        for value in [rect.left(), rect.right(), rect.top(), rect.bottom()] {
            radius = radius.max(value.abs());
        }
    }
    if let Some((x, _, r)) = shapes.circle_dot {
        radius = radius.max(x.abs() + r);
    }
    if let Some((r, t)) = shapes.ring {
        radius = radius.max(r + t / 2.0);
    }
    let mut total = radius + outline;
    if c.rotation.abs() > f32::EPSILON {
        total *= std::f32::consts::SQRT_2;
    }
    total
}

/// Dibuja el crosshair. Sin partes visibles devuelve [`CrosshairError::Empty`].
pub fn render(crosshair: &Crosshair) -> Result<Rendered, CrosshairError> {
    let c = crosshair.clamped();
    let s = c.scale / 100.0;
    let rotated = c.rotation.abs() > f32::EPSILON;

    if c.kind == CrosshairKind::Image {
        let image = decode_image(c.image.as_deref().unwrap_or_default())?;
        let half = (image.width().max(image.height()) as f32 * s) / 2.0;
        let radius = (if rotated {
            half * std::f32::consts::SQRT_2
        } else {
            half
        })
        .ceil() as u32
            + 2;
        let size = radius * 2 + 1;
        let mut pixmap = Pixmap::new(size, size).ok_or(CrosshairError::Empty)?;
        let center = radius as f32 + 0.5;
        let transform = Transform::from_translate(center, center)
            .pre_rotate(c.rotation)
            .pre_scale(s, s)
            .pre_translate(
                -(image.width() as f32) / 2.0,
                -(image.height() as f32) / 2.0,
            );
        let paint = PixmapPaint {
            opacity: c.opacity / 100.0,
            ..PixmapPaint::default()
        };
        pixmap.draw_pixmap(0, 0, image.as_ref(), &paint, transform, None);
        return Ok(Rendered {
            size,
            center: radius,
            rgba: pixmap.take(),
        });
    }

    let shapes = shapes(&c, s);
    if shapes.rects.is_empty() && shapes.circle_dot.is_none() && shapes.ring.is_none() {
        return Err(CrosshairError::Empty);
    }
    let outline = if c.outline.enabled {
        (c.outline.thickness * s).round().max(1.0)
    } else {
        0.0
    };
    let radius = extent(&c, &shapes, outline).ceil() as u32 + 2;
    let size = radius * 2 + 1;
    let mut pixmap = Pixmap::new(size, size).ok_or(CrosshairError::Empty)?;
    let center = radius as f32 + 0.5;
    let transform = Transform::from_translate(center, center).pre_rotate(c.rotation);

    let mut draw = |color: Color, grow: f32, blend: BlendMode| {
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.blend_mode = blend;
        paint.anti_alias = rotated;
        for rect in &shapes.rects {
            if let Some(rect) = Rect::from_ltrb(
                rect.left() - grow,
                rect.top() - grow,
                rect.right() + grow,
                rect.bottom() + grow,
            ) {
                pixmap.fill_rect(rect, &paint, transform, None);
            }
        }
        paint.anti_alias = true;
        if let Some((x, y, r)) = shapes.circle_dot
            && let Some(path) = PathBuilder::from_circle(x, y, r + grow)
        {
            pixmap.fill_path(&path, &paint, FillRule::Winding, transform, None);
        }
        if let Some((r, t)) = shapes.ring
            && let Some(path) = PathBuilder::from_circle(0.0, 0.0, r)
        {
            let stroke = Stroke {
                width: t + grow * 2.0,
                ..Stroke::default()
            };
            pixmap.stroke_path(&path, &paint, &stroke, transform, None);
        }
    };

    if outline > 0.0 {
        draw(
            parse_color(&c.outline.color, c.outline.opacity),
            outline,
            BlendMode::SourceOver,
        );
    }
    // El relleno reemplaza lo que hay debajo (su propio contorno).
    draw(parse_color(&c.color, c.opacity), 0.0, BlendMode::Source);

    Ok(Rendered {
        size,
        center: radius,
        rgba: pixmap.take(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opaque(rendered: &Rendered, x: u32, y: u32) -> bool {
        rendered.pixel(x, y)[3] == 255
    }

    #[test]
    fn classic_is_symmetric_and_has_a_hole_in_the_middle() {
        let rendered = render(&Crosshair::default()).expect("dibujable");
        assert_eq!(rendered.size % 2, 1, "hay un píxel central");
        let c = rendered.center;
        assert_eq!(
            rendered.pixel(c, c)[3],
            0,
            "sin punto, el centro está vacío"
        );
        // Brazo derecho: gap 3 → el relleno empieza en c + 4 y su contorno de
        // 1 px ocupa c + 3; c + 2 queda vacío.
        assert_eq!(rendered.pixel(c + 2, c)[3], 0);
        assert_eq!(rendered.pixel(c + 3, c), [0, 0, 0, 255]);
        assert!(opaque(&rendered, c + 4, c));
        assert!(opaque(&rendered, c + 9, c), "largo 6: hasta c + 9");
        // Simetría entre los cuatro brazos (con contorno incluido).
        for d in 0..=c {
            assert_eq!(rendered.pixel(c + d, c), rendered.pixel(c - d, c), "x ±{d}");
            assert_eq!(rendered.pixel(c, c + d), rendered.pixel(c, c - d), "y ±{d}");
        }
    }

    #[test]
    fn the_fill_color_wins_over_the_outline() {
        let crosshair = Crosshair {
            color: "#FF0000".into(),
            ..Crosshair::default()
        };
        let rendered = render(&crosshair).expect("dibujable");
        let c = rendered.center;
        assert_eq!(
            rendered.pixel(c + 5, c),
            [255, 0, 0, 255],
            "relleno rojo opaco"
        );
        let outline_row = c - 1; // grosor 2 (filas c y c + 1) + contorno de 1
        assert_eq!(
            rendered.pixel(c + 5, outline_row),
            [0, 0, 0, 255],
            "contorno negro"
        );
    }

    #[test]
    fn a_translucent_fill_is_not_darkened_by_its_outline() {
        let crosshair = Crosshair {
            color: "#FFFFFF".into(),
            opacity: 50.0,
            ..Crosshair::default()
        };
        let rendered = render(&crosshair).expect("dibujable");
        let c = rendered.center;
        let [r, g, b, a] = rendered.pixel(c + 5, c);
        assert_eq!(a, 128);
        assert_eq!((r, g, b), (128, 128, 128), "blanco al 50 % premultiplicado");
    }

    #[test]
    fn hidden_arms_are_not_drawn() {
        let crosshair = Crosshair {
            arms: Arms {
                top: false,
                ..Arms::default()
            },
            outline: Outline {
                enabled: false,
                ..Outline::default()
            },
            ..Crosshair::default()
        };
        let rendered = render(&crosshair).expect("dibujable");
        let c = rendered.center;
        assert!(opaque(&rendered, c, c + 5), "abajo sí");
        assert!(!opaque(&rendered, c, c - 5), "arriba no");
    }

    #[test]
    fn scale_grows_the_drawing() {
        let normal = render(&Crosshair::default()).expect("dibujable");
        let double = render(&Crosshair {
            scale: 200.0,
            ..Crosshair::default()
        })
        .expect("dibujable");
        assert!(double.size > normal.size);
        let c = double.center;
        assert!(
            opaque(&double, c + 18, c),
            "largo 12 + gap 6 a escala 200 %"
        );
    }

    #[test]
    fn every_preset_renders_something() {
        for preset in presets() {
            let rendered = render(&preset.crosshair).expect(&preset.id);
            assert!(
                rendered.rgba.as_chunks::<4>().0.iter().any(|px| px[3] > 0),
                "{} no dibuja nada",
                preset.id
            );
        }
    }

    #[test]
    fn nothing_enabled_is_an_error() {
        let crosshair = Crosshair {
            arms: Arms {
                enabled: false,
                ..Arms::default()
            },
            ..Crosshair::default()
        };
        assert_eq!(render(&crosshair), Err(CrosshairError::Empty));
    }

    #[test]
    fn image_crosshairs_must_be_png_data_urls() {
        let crosshair = Crosshair {
            kind: CrosshairKind::Image,
            image: Some("data:image/jpeg;base64,AAAA".into()),
            ..Crosshair::default()
        };
        assert_eq!(render(&crosshair), Err(CrosshairError::NotPng));
    }

    #[test]
    fn image_crosshairs_render_centered() {
        // PNG de 2 × 2 rojo opaco.
        let mut image = Pixmap::new(3, 3).expect("pixmap");
        image.fill(Color::from_rgba8(255, 0, 0, 255));
        let png = image.encode_png().expect("png");
        let mut encoded = String::new();
        const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        for chunk in png.chunks(3) {
            let b = [
                chunk[0],
                *chunk.get(1).unwrap_or(&0),
                *chunk.get(2).unwrap_or(&0),
            ];
            let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
            for i in 0..4 {
                if i <= chunk.len() {
                    encoded.push(TABLE[((n >> (18 - i * 6)) & 63) as usize] as char);
                } else {
                    encoded.push('=');
                }
            }
        }
        let crosshair = Crosshair {
            kind: CrosshairKind::Image,
            image: Some(format!("data:image/png;base64,{encoded}")),
            ..Crosshair::default()
        };
        let rendered = render(&crosshair).expect("dibujable");
        let c = rendered.center;
        assert_eq!(rendered.pixel(c, c), [255, 0, 0, 255]);
        assert_eq!(rendered.pixel(0, 0)[3], 0);
    }

    #[test]
    fn straight_and_bgra_conversions() {
        let rendered = Rendered {
            size: 1,
            center: 0,
            rgba: vec![64, 32, 0, 128],
        };
        assert_eq!(rendered.to_straight_rgba(), vec![128, 64, 0, 128]);
        assert_eq!(rendered.to_bgra_premultiplied(), vec![0, 32, 64, 128]);
    }

    #[test]
    fn clamped_keeps_values_in_range() {
        let wild = Crosshair {
            scale: 9000.0,
            offset_x: -999,
            opacity: f32::NAN,
            ..Crosshair::default()
        };
        let clean = wild.clamped();
        assert_eq!(clean.scale, 400.0);
        assert_eq!(clean.offset_x, -200);
        assert_eq!(clean.opacity, 100.0);
    }

    #[test]
    fn defaults_fill_missing_fields() {
        let crosshair: Crosshair = serde_json::from_str("{}").expect("válido");
        assert_eq!(crosshair, Crosshair::default());
    }
}
