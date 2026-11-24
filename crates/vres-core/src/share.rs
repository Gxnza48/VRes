//! Códigos para compartir: `VR<tipo><versión>-XXXXX-XXXXX-…`.
//!
//! - Tipo: `L` look, `C` crosshair, `R` resolución (`P` perfil, más adelante).
//! - Cuerpo: un byte de opciones, campos TLV y un CRC-32 calculado sobre el
//!   prefijo y el cuerpo (un código de un tipo no se puede leer como otro).
//! - TLV: clave = `(campo << 1) | tipo`; tipo 0 = entero (varint zigzag),
//!   tipo 1 = longitud + bytes. Los campos nunca se reutilizan y los
//!   desconocidos se saltean, así un código de una versión más nueva se lee
//!   igual.
//! - Solo viajan los campos distintos del valor por defecto, cuantizados con el
//!   paso de cada rango: un look típico ocupa unos pocos grupos.
//! - Alfabeto Base32 de Crockford en grupos de 5. Al pegar se normaliza:
//!   mayúsculas, sin espacios, guiones ni comillas, `O → 0` e `I`/`L → 1`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::crosshair::{Crosshair, CrosshairKind};
use crate::model::look::{Look, LumaFormula, VisionMode};
use crate::model::resolution::{ModeRequest, Scaling};

/// Versión del formato.
pub const VERSION: char = '1';
/// Largo máximo de un código, en caracteres.
pub const MAX_CODE_CHARS: usize = 2048;

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Qué hay en un código.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ShareKind {
    Look,
    Crosshair,
    Resolution,
}

impl ShareKind {
    fn letter(self) -> char {
        match self {
            Self::Look => 'L',
            Self::Crosshair => 'C',
            Self::Resolution => 'R',
        }
    }

    fn from_letter(letter: char) -> Option<Self> {
        match letter {
            'L' => Some(Self::Look),
            'C' => Some(Self::Crosshair),
            'R' => Some(Self::Resolution),
            _ => None,
        }
    }
}

/// Por qué no se pudo leer o armar un código.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, thiserror::Error)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ShareError {
    /// No empieza con `VR` y un tipo conocido.
    #[error("not a VRes code")]
    BadPrefix,
    /// Tiene caracteres que no son del alfabeto.
    #[error("invalid character")]
    BadChar,
    /// Se copió mal: la suma de control no coincide.
    #[error("checksum mismatch")]
    BadChecksum,
    /// Es de una versión más nueva de VRes.
    #[error("unsupported version")]
    UnsupportedVersion,
    /// Es de otro tipo (por ejemplo, un crosshair donde va un look).
    #[error("wrong kind")]
    WrongKind,
    /// Es demasiado largo.
    #[error("code too long")]
    TooLong,
    /// Está cortado.
    #[error("code truncated")]
    Truncated,
    /// Los crosshairs de imagen no entran en un código.
    #[error("image crosshairs cannot be shared")]
    ImageNotShareable,
}

/// Lo que se leyó de un código.
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded<T> {
    pub value: T,
    /// Algún valor estaba fuera de rango y se recortó.
    pub clamped: bool,
}

// --------------------------------------------------------------- Base32 ---

fn encode_base32(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 8 / 5 + 1);
    let (mut buffer, mut bits) = (0u32, 0u32);
    for &byte in bytes {
        buffer = (buffer << 8) | u32::from(byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(ALPHABET[((buffer >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(ALPHABET[((buffer << (5 - bits)) & 31) as usize] as char);
    }
    out
}

fn decode_base32(text: &str) -> Result<Vec<u8>, ShareError> {
    let mut out = Vec::with_capacity(text.len() * 5 / 8);
    let (mut buffer, mut bits) = (0u32, 0u32);
    for c in text.bytes() {
        let value = ALPHABET
            .iter()
            .position(|&a| a == c)
            .ok_or(ShareError::BadChar)? as u32;
        buffer = (buffer << 5) | value;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }
    Ok(out)
}

/// Normaliza un código pegado (de un chat, con espacios o mal transcripto).
/// Es idempotente.
pub fn normalize(code: &str) -> String {
    let compact: String = code
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    // El prefijo (`VRL1`) tiene letras que el cuerpo no usa: se respeta tal cual.
    let split = compact.len().min(4);
    let (prefix, body) = compact.split_at(split);
    let body: String = body
        .chars()
        .map(|c| match c {
            'O' => '0',
            'I' | 'L' => '1',
            other => other,
        })
        .collect();
    format!("{prefix}{body}")
}

// --------------------------------------------------------------- CRC-32 ---

fn crc32(chunks: &[&[u8]]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for chunk in chunks {
        for &byte in *chunk {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                let mask = (crc & 1).wrapping_neg();
                crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
            }
        }
    }
    !crc
}

// ------------------------------------------------------------------ TLV ---

#[derive(Default)]
struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    fn varint(&mut self, mut value: u64) {
        loop {
            let byte = (value & 0x7F) as u8;
            value >>= 7;
            if value == 0 {
                self.bytes.push(byte);
                return;
            }
            self.bytes.push(byte | 0x80);
        }
    }

    /// Un entero; si es 0 (igual al valor por defecto) no se escribe.
    fn int(&mut self, field: u8, value: i64) {
        if value == 0 {
            return;
        }
        self.bytes.push(field << 1);
        self.varint(((value << 1) ^ (value >> 63)) as u64);
    }
}

enum Value {
    Int(i64),
    Bytes,
}

fn read_varint(bytes: &[u8], at: &mut usize) -> Result<u64, ShareError> {
    let mut value = 0u64;
    for shift in (0..64).step_by(7) {
        let byte = *bytes.get(*at).ok_or(ShareError::Truncated)?;
        *at += 1;
        value |= u64::from(byte & 0x7F) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(ShareError::Truncated)
}

/// Recorre los campos; los de tipo bytes (todavía no usados) se saltean.
fn read_fields(bytes: &[u8]) -> Result<Vec<(u8, Value)>, ShareError> {
    let mut fields = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let key = bytes[at];
        at += 1;
        let field = key >> 1;
        if key & 1 == 0 {
            let raw = read_varint(bytes, &mut at)?;
            let value = ((raw >> 1) as i64) ^ -((raw & 1) as i64);
            fields.push((field, Value::Int(value)));
        } else {
            let len = read_varint(bytes, &mut at)? as usize;
            at = at.checked_add(len).ok_or(ShareError::Truncated)?;
            if at > bytes.len() {
                return Err(ShareError::Truncated);
            }
            fields.push((field, Value::Bytes));
        }
    }
    Ok(fields)
}

fn wrap(kind: ShareKind, fields: Writer) -> String {
    let prefix = format!("VR{}{VERSION}", kind.letter());
    let mut body = vec![0u8];
    body.extend_from_slice(&fields.bytes);
    let crc = crc32(&[prefix.as_bytes(), &body]);
    body.extend_from_slice(&crc.to_le_bytes());
    let encoded = encode_base32(&body);
    let groups: Vec<&str> = encoded
        .as_bytes()
        .chunks(5)
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or_default())
        .collect();
    format!("{prefix}-{}", groups.join("-"))
}

/// Tipo de un código (sin validarlo entero).
pub fn kind_of(code: &str) -> Result<ShareKind, ShareError> {
    let normalized = normalize(code);
    let mut chars = normalized.chars();
    if chars.next() != Some('V') || chars.next() != Some('R') {
        return Err(ShareError::BadPrefix);
    }
    chars
        .next()
        .and_then(ShareKind::from_letter)
        .ok_or(ShareError::BadPrefix)
}

fn unwrap(code: &str, expected: ShareKind) -> Result<Vec<(u8, Value)>, ShareError> {
    if code.len() > MAX_CODE_CHARS {
        return Err(ShareError::TooLong);
    }
    let normalized = normalize(code);
    if kind_of(&normalized)? != expected {
        return Err(ShareError::WrongKind);
    }
    if normalized.chars().nth(3) != Some(VERSION) {
        return Err(ShareError::UnsupportedVersion);
    }
    let (prefix, payload) = normalized.split_at(4);
    let bytes = decode_base32(payload)?;
    if bytes.len() < 5 {
        return Err(ShareError::Truncated);
    }
    let (body, crc) = bytes.split_at(bytes.len() - 4);
    let expected_crc = u32::from_le_bytes([crc[0], crc[1], crc[2], crc[3]]);
    if crc32(&[prefix.as_bytes(), body]) != expected_crc {
        return Err(ShareError::BadChecksum);
    }
    read_fields(&body[1..])
}

fn q(value: f32, scale: f32) -> i64 {
    (value * scale).round() as i64
}

// ----------------------------------------------------------------- Look ---

/// Campos del look: (id, cómo leerlo y escribirlo, escala).
type LookField = (u8, fn(&mut Look) -> &mut f32, f32);

const LOOK_FIELDS: &[LookField] = &[
    (1, |l| &mut l.saturation, 1.0),
    (2, |l| &mut l.vibrance, 1.0),
    (3, |l| &mut l.tone.brightness, 1.0),
    (4, |l| &mut l.tone.contrast, 1.0),
    (5, |l| &mut l.tone.gamma, 100.0),
    (6, |l| &mut l.tone.temperature, 1.0),
    (7, |l| &mut l.tone.highlights, 1.0),
    (8, |l| &mut l.tone.shadows, 1.0),
    (9, |l| &mut l.tone.blacks, 1.0),
    (10, |l| &mut l.tone.whites, 1.0),
    (11, |l| &mut l.tone.shadow_tint, 1.0),
    (12, |l| &mut l.tone.highlight_tint, 1.0),
    (13, |l| &mut l.tone.gain[0], 100.0),
    (14, |l| &mut l.tone.gain[1], 100.0),
    (15, |l| &mut l.tone.gain[2], 100.0),
    (17, |l| &mut l.color.hue_shift_deg, 1.0),
    (18, |l| &mut l.color.hue_shift_amount, 1.0),
    (19, |l| &mut l.color.darken_hue_deg, 1.0),
    (20, |l| &mut l.color.darken_amount, 1.0),
    (22, |l| &mut l.vision.strength, 1.0),
    (23, |l| &mut l.eye_care.warmth, 1.0),
    (24, |l| &mut l.eye_care.dim, 1.0),
];
const LOOK_LUMA: u8 = 16;
const LOOK_VISION: u8 = 21;

fn luma_index(luma: LumaFormula) -> i64 {
    match luma {
        LumaFormula::Rec709 => 0,
        LumaFormula::Rec601 => 1,
        LumaFormula::Rec2020 => 2,
        LumaFormula::Average => 3,
    }
}

fn vision_index(mode: VisionMode) -> i64 {
    match mode {
        VisionMode::Off => 0,
        VisionMode::Protan => 1,
        VisionMode::Deutan => 2,
        VisionMode::Tritan => 3,
    }
}

/// Código de un look.
pub fn encode_look(look: &Look) -> String {
    let mut look = look.clamped();
    let mut base = Look::default();
    let mut writer = Writer::default();
    for (id, get, scale) in LOOK_FIELDS {
        writer.int(*id, q(*get(&mut look), *scale) - q(*get(&mut base), *scale));
    }
    writer.int(LOOK_LUMA, luma_index(look.color.luma));
    writer.int(LOOK_VISION, vision_index(look.vision.mode));
    wrap(ShareKind::Look, writer)
}

/// Lee el código de un look.
pub fn decode_look(code: &str) -> Result<Decoded<Look>, ShareError> {
    let fields = unwrap(code, ShareKind::Look)?;
    let mut look = Look::default();
    for (id, value) in fields {
        let Value::Int(delta) = value else { continue };
        if let Some((_, get, scale)) = LOOK_FIELDS.iter().find(|(field, ..)| *field == id) {
            let slot = get(&mut look);
            *slot = (q(*slot, *scale) + delta) as f32 / scale;
        } else if id == LOOK_LUMA {
            look.color.luma = match delta {
                1 => LumaFormula::Rec601,
                2 => LumaFormula::Rec2020,
                3 => LumaFormula::Average,
                _ => LumaFormula::Rec709,
            };
        } else if id == LOOK_VISION {
            look.vision.mode = match delta {
                1 => VisionMode::Protan,
                2 => VisionMode::Deutan,
                3 => VisionMode::Tritan,
                _ => VisionMode::Off,
            };
        }
    }
    let clamped_look = look.clamped();
    let clamped = clamped_look != look;
    Ok(Decoded {
        value: clamped_look,
        clamped,
    })
}

// ------------------------------------------------------------ Crosshair ---

fn rgb(hex: &str) -> i64 {
    i64::from_str_radix(hex.trim().trim_start_matches('#'), 16).unwrap_or(0xFF_FFFF) & 0xFF_FFFF
}

fn hex(value: i64) -> String {
    format!("#{:06X}", value.clamp(0, 0xFF_FFFF))
}

fn sides(c: &Crosshair) -> i64 {
    i64::from(c.arms.top)
        | i64::from(c.arms.right) << 1
        | i64::from(c.arms.bottom) << 2
        | i64::from(c.arms.left) << 3
}

/// Campos numéricos del crosshair: (id, cómo leerlo y escribirlo).
type CrosshairField = (u8, fn(&mut Crosshair) -> &mut f32);

const CROSSHAIR_FIELDS: &[CrosshairField] = &[
    (2, |c| &mut c.opacity),
    (5, |c| &mut c.outline.thickness),
    (6, |c| &mut c.outline.opacity),
    (8, |c| &mut c.arms.length),
    (9, |c| &mut c.arms.thickness),
    (10, |c| &mut c.arms.gap),
    (13, |c| &mut c.dot.size),
    (16, |c| &mut c.ring.radius),
    (17, |c| &mut c.ring.thickness),
    (18, |c| &mut c.rotation),
    (19, |c| &mut c.scale),
];

/// Interruptores del crosshair: (id, cómo leerlo y escribirlo).
type CrosshairFlag = (u8, fn(&mut Crosshair) -> &mut bool);

const CROSSHAIR_FLAGS: &[CrosshairFlag] = &[
    (3, |c| &mut c.outline.enabled),
    (7, |c| &mut c.arms.enabled),
    (12, |c| &mut c.dot.enabled),
    (14, |c| &mut c.dot.round),
    (15, |c| &mut c.ring.enabled),
];

/// Código de un crosshair. Los de imagen no se pueden compartir.
pub fn encode_crosshair(crosshair: &Crosshair) -> Result<String, ShareError> {
    if crosshair.kind == CrosshairKind::Image {
        return Err(ShareError::ImageNotShareable);
    }
    let mut c = crosshair.clamped();
    let mut base = Crosshair::default();
    let mut writer = Writer::default();
    writer.int(1, rgb(&c.color) - rgb(&base.color));
    writer.int(4, rgb(&c.outline.color) - rgb(&base.outline.color));
    for (id, get) in CROSSHAIR_FIELDS {
        writer.int(*id, q(*get(&mut c), 1.0) - q(*get(&mut base), 1.0));
    }
    for (id, get) in CROSSHAIR_FLAGS {
        writer.int(*id, i64::from(*get(&mut c)) - i64::from(*get(&mut base)));
    }
    writer.int(11, sides(&c) - sides(&base));
    writer.int(20, i64::from(c.offset_x - base.offset_x));
    writer.int(21, i64::from(c.offset_y - base.offset_y));
    Ok(wrap(ShareKind::Crosshair, writer))
}

/// Lee el código de un crosshair.
pub fn decode_crosshair(code: &str) -> Result<Decoded<Crosshair>, ShareError> {
    let fields = unwrap(code, ShareKind::Crosshair)?;
    let mut c = Crosshair::default();
    for (id, value) in fields {
        let Value::Int(delta) = value else { continue };
        if let Some((_, get)) = CROSSHAIR_FIELDS.iter().find(|(field, _)| *field == id) {
            let slot = get(&mut c);
            *slot = (q(*slot, 1.0) + delta) as f32;
        } else if let Some((_, get)) = CROSSHAIR_FLAGS.iter().find(|(field, _)| *field == id) {
            let slot = get(&mut c);
            *slot = i64::from(*slot) + delta != 0;
        } else {
            match id {
                1 => c.color = hex(rgb(&c.color) + delta),
                4 => c.outline.color = hex(rgb(&c.outline.color) + delta),
                11 => {
                    let mask = sides(&c) + delta;
                    c.arms.top = mask & 1 != 0;
                    c.arms.right = mask & 2 != 0;
                    c.arms.bottom = mask & 4 != 0;
                    c.arms.left = mask & 8 != 0;
                }
                20 => c.offset_x = (i64::from(c.offset_x) + delta).clamp(-10_000, 10_000) as i32,
                21 => c.offset_y = (i64::from(c.offset_y) + delta).clamp(-10_000, 10_000) as i32,
                _ => {}
            }
        }
    }
    let clamped_value = c.clamped();
    let clamped = clamped_value != c;
    Ok(Decoded {
        value: clamped_value,
        clamped,
    })
}

// ----------------------------------------------------------- Resolución ---

fn scaling_index(scaling: Scaling) -> i64 {
    match scaling {
        Scaling::Preferred => 0,
        Scaling::Identity => 1,
        Scaling::Centered => 2,
        Scaling::Stretched => 3,
        Scaling::AspectRatio => 4,
        Scaling::Custom => 5,
    }
}

/// Código de una resolución.
pub fn encode_resolution(mode: &ModeRequest) -> String {
    let mut writer = Writer::default();
    writer.int(1, i64::from(mode.width));
    writer.int(2, i64::from(mode.height));
    writer.int(3, i64::from(mode.refresh_hz));
    writer.int(4, scaling_index(mode.scaling));
    wrap(ShareKind::Resolution, writer)
}

/// Lee el código de una resolución.
pub fn decode_resolution(code: &str) -> Result<Decoded<ModeRequest>, ShareError> {
    let fields = unwrap(code, ShareKind::Resolution)?;
    let mut mode = ModeRequest {
        width: 0,
        height: 0,
        refresh_hz: 0,
        scaling: Scaling::Preferred,
    };
    for (id, value) in fields {
        let Value::Int(value) = value else { continue };
        let number = value.clamp(0, 100_000) as u32;
        match id {
            1 => mode.width = number,
            2 => mode.height = number,
            3 => mode.refresh_hz = number,
            4 => {
                mode.scaling = match value {
                    1 => Scaling::Identity,
                    2 => Scaling::Centered,
                    3 => Scaling::Stretched,
                    4 => Scaling::AspectRatio,
                    5 => Scaling::Custom,
                    _ => Scaling::Preferred,
                };
            }
            _ => {}
        }
    }
    if mode.width == 0 || mode.height == 0 {
        return Err(ShareError::Truncated);
    }
    Ok(Decoded {
        value: mode,
        clamped: false,
    })
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)] // los casos de prueba se arman campo por campo
mod tests {
    use super::*;
    use crate::crosshair::{Arms, Dot};
    use proptest::prelude::*;

    fn vivid() -> Look {
        let mut look = Look::default();
        look.saturation = 165.0;
        look.vibrance = 70.0;
        look.tone.gamma = 1.12;
        look.tone.shadows = 25.0;
        look.tone.gain = [1.05, 1.0, 0.97];
        look.color.luma = LumaFormula::Rec601;
        look.vision.mode = VisionMode::Deutan;
        look.vision.strength = 80.0;
        look
    }

    #[test]
    fn a_look_round_trips_and_stays_short() {
        let code = encode_look(&vivid());
        assert!(code.starts_with("VRL1-"), "{code}");
        assert!(code.len() < 60, "un look típico es corto: {code}");
        let decoded = decode_look(&code).expect("válido");
        assert_eq!(decoded.value, vivid());
        assert!(!decoded.clamped);
    }

    #[test]
    fn the_default_look_is_tiny() {
        let code = encode_look(&Look::default());
        assert_eq!(decode_look(&code).expect("válido").value, Look::default());
        assert!(code.len() <= 15, "{code}");
    }

    #[test]
    fn pasted_codes_are_forgiven() {
        let code = encode_look(&vivid());
        let messy = format!(" \"{}\"\u{200B}\n", code.to_lowercase().replace('-', " - "));
        assert_eq!(decode_look(&messy).expect("se normaliza").value, vivid());
        let body = &code[5..];
        let mistyped = format!("VRL1-{}", body.replace('0', "O").replace('1', "l"));
        assert_eq!(decode_look(&mistyped).expect("O→0, l→1").value, vivid());
    }

    #[test]
    fn normalize_is_idempotent() {
        for code in [
            encode_look(&vivid()),
            "vrl1-ooiil 123".into(),
            String::new(),
        ] {
            let once = normalize(&code);
            assert_eq!(normalize(&once), once);
        }
    }

    #[test]
    fn a_corrupted_character_is_detected() {
        let code = encode_look(&vivid());
        let mut chars: Vec<char> = code.chars().collect();
        let last = chars.len() - 3;
        chars[last] = if chars[last] == 'A' { 'B' } else { 'A' };
        let broken: String = chars.into_iter().collect();
        assert!(matches!(
            decode_look(&broken),
            Err(ShareError::BadChecksum | ShareError::Truncated)
        ));
        assert_eq!(
            decode_look("VRL1-UUUUU"),
            Err(ShareError::BadChar),
            "la U no es del alfabeto"
        );
    }

    #[test]
    fn kinds_and_versions_are_checked() {
        let look = encode_look(&vivid());
        assert_eq!(decode_crosshair(&look), Err(ShareError::WrongKind));
        assert_eq!(decode_look("hola"), Err(ShareError::BadPrefix));
        assert_eq!(
            decode_look(&look.replacen("VRL1", "VRL2", 1)),
            Err(ShareError::UnsupportedVersion)
        );
        assert_eq!(kind_of(&look), Ok(ShareKind::Look));
    }

    #[test]
    fn unknown_fields_are_skipped() {
        let mut writer = Writer::default();
        writer.int(1, 30); // saturación 130
        writer.int(99, 12345); // campo de una versión futura
        writer
            .bytes
            .extend_from_slice(&[(98 << 1) | 1, 2, 0xAB, 0xCD]); // bytes desconocidos
        let code = wrap(ShareKind::Look, writer);
        let decoded = decode_look(&code).expect("se lee igual");
        assert_eq!(decoded.value.saturation, 130.0);
    }

    #[test]
    fn crosshairs_round_trip_and_images_are_refused() {
        let crosshair = Crosshair {
            color: "#00FFCC".into(),
            arms: Arms {
                length: 9.0,
                top: false,
                ..Arms::default()
            },
            dot: Dot {
                enabled: true,
                size: 3.0,
                round: true,
            },
            rotation: 45.0,
            offset_x: -12,
            ..Crosshair::default()
        };
        let code = encode_crosshair(&crosshair).expect("vectorial");
        assert!(code.starts_with("VRC1-"));
        assert_eq!(decode_crosshair(&code).expect("válido").value, crosshair);
        let image = Crosshair {
            kind: CrosshairKind::Image,
            ..Crosshair::default()
        };
        assert_eq!(encode_crosshair(&image), Err(ShareError::ImageNotShareable));
    }

    #[test]
    fn resolutions_round_trip() {
        let mode = ModeRequest {
            width: 1440,
            height: 1080,
            refresh_hz: 200,
            scaling: Scaling::Stretched,
        };
        let code = encode_resolution(&mode);
        assert!(code.starts_with("VRR1-"));
        assert_eq!(decode_resolution(&code).expect("válido").value, mode);
    }

    #[test]
    fn out_of_range_values_are_clamped_and_reported() {
        let mut writer = Writer::default();
        writer.int(1, 5000); // saturación 5100 %
        let decoded = decode_look(&wrap(ShareKind::Look, writer)).expect("se lee");
        assert_eq!(decoded.value.saturation, 300.0);
        assert!(decoded.clamped);
    }

    proptest! {
        #[test]
        fn any_quantized_look_round_trips(
            saturation in 0i32..=300,
            vibrance in 0i32..=100,
            brightness in 50i32..=170,
            gamma in 50i32..=150,
            tonal in -100i32..=100,
            gain in 50i32..=150,
            hue in -180i32..=180,
            dim in 0i32..=80,
            luma in 0usize..4,
            vision in 0usize..4,
        ) {
            let mut look = Look::default();
            look.saturation = saturation as f32;
            look.vibrance = vibrance as f32;
            look.tone.brightness = brightness as f32;
            look.tone.gamma = gamma as f32 / 100.0;
            look.tone.shadows = tonal as f32;
            look.tone.highlight_tint = (-tonal) as f32;
            look.tone.gain = [gain as f32 / 100.0, 1.0, (200 - gain) as f32 / 100.0];
            look.color.hue_shift_deg = hue as f32;
            look.eye_care.dim = dim as f32;
            look.color.luma = [LumaFormula::Rec709, LumaFormula::Rec601, LumaFormula::Rec2020, LumaFormula::Average][luma];
            look.vision.mode = [VisionMode::Off, VisionMode::Protan, VisionMode::Deutan, VisionMode::Tritan][vision];
            let decoded = decode_look(&encode_look(&look)).expect("válido");
            prop_assert_eq!(decoded.value, look);
        }

        #[test]
        fn any_crosshair_round_trips(
            color in 0u32..=0xFF_FFFF,
            length in 0i32..=80,
            thickness in 1i32..=20,
            gap in 0i32..=40,
            sides in 0u8..16,
            dot in any::<bool>(),
            ring in any::<bool>(),
            rotation in -180i32..=180,
            scale in 5i32..=80,
            offset in -200i32..=200,
        ) {
            let mut c = Crosshair::default();
            c.color = format!("#{color:06X}");
            c.arms.length = length as f32;
            c.arms.thickness = thickness as f32;
            c.arms.gap = gap as f32;
            c.arms.top = sides & 1 != 0;
            c.arms.right = sides & 2 != 0;
            c.arms.bottom = sides & 4 != 0;
            c.arms.left = sides & 8 != 0;
            c.dot.enabled = dot;
            c.ring.enabled = ring;
            c.rotation = rotation as f32;
            c.scale = (scale * 5) as f32;
            c.offset_x = offset;
            c.offset_y = -offset;
            let code = encode_crosshair(&c).expect("vectorial");
            prop_assert_eq!(decode_crosshair(&code).expect("válido").value, c);
        }
    }
}
