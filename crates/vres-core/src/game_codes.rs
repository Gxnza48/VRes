//! Códigos de mira de otros juegos: VALORANT y Counter-Strike 2.
//!
//! Se traducen al modelo de VRes (brazos, punto y contorno). Es una
//! **aproximación**: VRes dibuja un solo juego de brazos, así que de VALORANT
//! se toman las líneas interiores (o las exteriores si las interiores están
//! apagadas), y los tamaños de CS2 se pasan a píxeles de una pantalla de 1080.
//!
//! Formato de CS2: base 57 con el alfabeto de Valve, suma de control en el
//! primer byte; disposición de los bytes según `akiver/csgo-sharecode` (MIT),
//! ver `THIRD-PARTY-NOTICES.md`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::crosshair::{Arms, Crosshair, Dot, Outline, Ring};

/// De qué juego vino una mira importada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum GameCodeSource {
    Valorant,
    Cs2,
}

/// Por qué no se pudo leer un código de otro juego.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GameCodeError {
    #[error("invalid character")]
    BadChar,
    #[error("checksum mismatch")]
    BadChecksum,
    #[error("unsupported version")]
    UnsupportedVersion,
}

/// Lee un código de VALORANT o de CS2. `None` si no parece de ninguno.
pub fn decode(code: &str) -> Option<Result<(GameCodeSource, Crosshair), GameCodeError>> {
    let trimmed = code
        .trim()
        .trim_matches(|c| c == '"' || c == '\'' || c == '`');
    let compact: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
    if let Some(body) = compact.strip_prefix("CSGO") {
        let chars: String = body.chars().filter(|&c| c != '-').collect();
        return Some(cs2_legacy(&chars).map(|c| (GameCodeSource::Cs2, c)));
    }
    if compact.len() == 46 && compact.starts_with("CS") {
        return Some(cs2_v1(&compact[2..]).map(|c| (GameCodeSource::Cs2, c)));
    }
    if looks_like_valorant(&compact) {
        return Some(Ok((GameCodeSource::Valorant, valorant(&compact))));
    }
    None
}

// ------------------------------------------------------------- VALORANT ---

fn looks_like_valorant(code: &str) -> bool {
    code.starts_with("0;") && code.split(';').any(|token| token == "P")
}

/// Colores fijos de VALORANT (índice `c`).
const VALORANT_COLORS: [&str; 8] = [
    "#FFFFFF", "#00FF00", "#7FFF00", "#DFFF00", "#FFFF00", "#00FFFF", "#FF00FF", "#FF0000",
];

/// Mira principal de un código de VALORANT. Las claves que faltan toman el
/// valor de la mira por defecto del juego.
fn valorant(code: &str) -> Crosshair {
    let mut values = std::collections::HashMap::new();
    let tokens: Vec<&str> = code.split(';').collect();
    let mut section = "";
    let mut index = 1; // el primer "0" es la versión
    while index < tokens.len() {
        let token = tokens[index];
        if matches!(token, "P" | "A" | "S") {
            section = token;
            index += 1;
            continue;
        }
        if section == "P"
            && let Some(value) = tokens.get(index + 1)
        {
            values.insert(token, *value);
        }
        index += 2;
    }
    let number = |key: &str, default: f32| {
        values
            .get(key)
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(default)
    };
    let flag = |key: &str, default: bool| values.get(key).map_or(default, |v| *v != "0");

    let color = match values.get("c").and_then(|v| v.parse::<usize>().ok()) {
        Some(8) => values
            .get("u")
            .filter(|hex| hex.len() >= 6 && hex.chars().all(|c| c.is_ascii_hexdigit()))
            .map(|hex| format!("#{}", hex[..6].to_ascii_uppercase()))
            .unwrap_or_else(|| VALORANT_COLORS[0].into()),
        Some(i) if i < VALORANT_COLORS.len() => VALORANT_COLORS[i].into(),
        _ => VALORANT_COLORS[0].into(),
    };

    // Líneas interiores (por defecto: 6 de largo, 2 de grosor, 3 de separación)
    // o, si están apagadas, las exteriores.
    let inner = flag("0b", true) && number("0l", 6.0) > 0.0;
    let outer = flag("1b", true) && number("1l", 2.0) > 0.0;
    let (prefix, length, thickness, gap, alpha) = if inner || !outer {
        ("0", 6.0, 2.0, 3.0, 0.8)
    } else {
        ("1", 2.0, 2.0, 10.0, 0.35)
    };
    let key = |suffix: &str| format!("{prefix}{suffix}");
    let arms_on = inner || outer;
    let dot_on = flag("d", false);
    let line_alpha = number(&key("a"), alpha);
    let opacity = if arms_on {
        line_alpha
    } else {
        number("a", 1.0)
    };

    Crosshair {
        color,
        opacity: (opacity * 100.0).round(),
        outline: Outline {
            enabled: flag("h", true),
            color: "#000000".into(),
            thickness: number("t", 1.0).max(1.0),
            opacity: (number("o", 0.5) * 100.0).round(),
        },
        arms: Arms {
            enabled: arms_on,
            length: number(&key("l"), length),
            thickness: number(&key("t"), thickness).max(1.0),
            gap: number(&key("o"), gap),
            ..Arms::default()
        },
        dot: Dot {
            enabled: dot_on,
            size: number("z", 2.0).max(1.0),
            round: false,
        },
        ring: Ring::default(),
        ..Crosshair::default()
    }
    .clamped()
}

// ------------------------------------------------------------------ CS2 ---

const CS_ALPHABET: &str = "ABCDEFGHJKLMNOPQRSTUVWXYZabcdefhijkmnopqrstuvwxyz23456789";

/// Base 57 (primer carácter = dígito menos significativo) → `length` bytes
/// en orden big-endian, con la suma de control verificada.
fn cs_bytes(chars: &str, length: usize) -> Result<Vec<u8>, GameCodeError> {
    let mut bytes = vec![0u8; length]; // big-endian
    for c in chars.chars().rev() {
        let digit = CS_ALPHABET.find(c).ok_or(GameCodeError::BadChar)? as u32;
        // bytes = bytes * 57 + digit
        let mut carry = digit;
        for byte in bytes.iter_mut().rev() {
            let value = u32::from(*byte) * 57 + carry;
            *byte = (value & 0xFF) as u8;
            carry = value >> 8;
        }
        if carry != 0 {
            return Err(GameCodeError::BadChecksum);
        }
    }
    let sum = bytes[1..].iter().map(|&b| u32::from(b)).sum::<u32>() % 256;
    if u32::from(bytes[0]) != sum {
        return Err(GameCodeError::BadChecksum);
    }
    Ok(bytes)
}

fn hex(r: u8, g: u8, b: u8) -> String {
    format!("#{r:02X}{g:02X}{b:02X}")
}

/// Medidas en píxeles de una pantalla de `height` → píxeles a 1080.
fn at_1080(value: f32, height: f32) -> f32 {
    (value * 1080.0 / height.max(240.0)).round()
}

struct CsShape {
    color: String,
    alpha: f32,
    /// Color, opacidad (0–1) y grosor en píxeles.
    outline: Option<(String, f32, f32)>,
    gap: f32,
    length: f32,
    thickness: f32,
    dot: bool,
    t_style: bool,
}

impl CsShape {
    fn build(self) -> Crosshair {
        let thickness = self.thickness.max(1.0);
        Crosshair {
            color: self.color,
            opacity: (self.alpha * 100.0).round(),
            outline: match self.outline {
                Some((color, opacity, thickness)) => Outline {
                    enabled: true,
                    color,
                    thickness: thickness.round().max(1.0),
                    opacity: (opacity * 100.0).round(),
                },
                None => Outline {
                    enabled: false,
                    ..Outline::default()
                },
            },
            arms: Arms {
                enabled: self.length > 0.0,
                length: self.length,
                thickness,
                gap: self.gap.max(0.0),
                top: !self.t_style,
                ..Arms::default()
            },
            dot: Dot {
                enabled: self.dot,
                size: thickness,
                round: false,
            },
            ring: Ring::default(),
            ..Crosshair::default()
        }
        .clamped()
    }
}

/// `CSGO-…`: versiones 1 (valores de las ConVars), 3 y 4 (en píxeles).
fn cs2_legacy(chars: &str) -> Result<Crosshair, GameCodeError> {
    if chars.chars().count() != 25 {
        return Err(GameCodeError::BadChar);
    }
    let b = cs_bytes(chars, 18)?;
    match b[1] {
        1 => {
            // ConVars clásicas; aproximación a píxeles de 1080.
            let gap = f32::from(b[2] as i8) / 10.0;
            let outline = f32::from(b[3]) / 2.0;
            let color = match b[10] & 7 {
                0 => hex(250, 50, 50),
                1 => hex(50, 250, 50),
                2 => hex(250, 250, 50),
                3 => hex(50, 50, 250),
                4 => hex(50, 250, 250),
                _ => hex(b[4], b[5], b[6]),
            };
            let flags = b[13] >> 4;
            let alpha = if flags & 4 == 4 {
                f32::from(b[7]) / 255.0
            } else {
                1.0
            };
            Ok(CsShape {
                color,
                alpha,
                outline: (b[10] & 8 == 8).then(|| ("#000000".to_owned(), 1.0, outline)),
                gap: (gap + 4.0).round(),
                length: (f32::from(b[14]) / 10.0 * 2.0).round(),
                thickness: (f32::from(b[12]) / 10.0 * 2.0).round(),
                dot: flags & 1 == 1,
                t_style: flags & 8 == 8,
            }
            .build())
        }
        3 | 4 => {
            let bits = u32::from_le_bytes([b[10], b[11], b[12], b[13]]);
            let height = f32::from(u16::from_le_bytes([b[14], b[15]]));
            let outline = if b[1] == 3 {
                b[2] & 0x20 == 0x20
            } else {
                (b[13] >> 4) & 3 != 0
            };
            Ok(CsShape {
                color: hex(b[3], b[4], b[5]),
                alpha: f32::from(b[6]) / 255.0,
                outline: outline.then(|| ("#000000".to_owned(), 1.0, 1.0)),
                gap: at_1080(f32::from(b[7]), height),
                length: at_1080(f32::from(b[8]), height),
                thickness: at_1080(((bits >> 23) & 0x1F) as f32, height),
                dot: b[2] & 0x40 == 0x40,
                t_style: b[2] & 0x80 == 0x80,
            }
            .build())
        }
        _ => Err(GameCodeError::UnsupportedVersion),
    }
}

/// `CS…` (44 caracteres, formato de CS2 desde fines de 2026).
fn cs2_v1(chars: &str) -> Result<Crosshair, GameCodeError> {
    let b = cs_bytes(chars, 32)?;
    if b[1] != 1 {
        return Err(GameCodeError::UnsupportedVersion);
    }
    let height = f32::from(u16::from_le_bytes([b[2], b[3]]));
    if height == 0.0 {
        return Err(GameCodeError::BadChecksum);
    }
    let gap = f32::from(i16::from_le_bytes([b[14], b[15]])).clamp(-3840.0, 3840.0);
    let outline_mode = (b[13] >> 6).min(2);
    Ok(CsShape {
        color: hex(b[5], b[6], b[7]),
        alpha: f32::from(b[8]) / 255.0,
        outline: (outline_mode != 0)
            .then(|| (hex(b[9], b[10], b[11]), f32::from(b[12]) / 255.0, 1.0)),
        gap: at_1080(gap, height),
        length: at_1080(f32::from(b[16]), height),
        thickness: at_1080(f32::from((b[13] & 0x3F).min(32)), height),
        dot: b[4] & 0x40 == 0x40,
        t_style: b[4] & 0x80 == 0x80,
    }
    .build())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Arma un código `CSGO-` con estos bytes (para probar la lectura).
    fn encode_legacy(mut bytes: [u8; 18]) -> String {
        bytes[0] = (bytes[1..].iter().map(|&b| u32::from(b)).sum::<u32>() % 256) as u8;
        let alphabet: Vec<char> = CS_ALPHABET.chars().collect();
        let mut number = bytes.to_vec();
        let mut chars = String::new();
        for _ in 0..25 {
            let mut remainder = 0u32;
            for byte in number.iter_mut() {
                let value = (remainder << 8) | u32::from(*byte);
                *byte = (value / 57) as u8;
                remainder = value % 57;
            }
            chars.push(alphabet[remainder as usize]);
        }
        let groups: Vec<String> = chars
            .as_bytes()
            .chunks(5)
            .map(|c| String::from_utf8_lossy(c).into_owned())
            .collect();
        format!("CSGO-{}", groups.join("-"))
    }

    #[test]
    fn valorant_reads_the_primary_crosshair() {
        let (source, c) = decode("0;P;c;5;h;0;0l;4;0o;2;0a;1;0f;0;1b;0")
            .unwrap()
            .unwrap();
        assert_eq!(source, GameCodeSource::Valorant);
        assert_eq!(c.color, "#00FFFF");
        assert!(!c.outline.enabled);
        assert!(c.arms.enabled);
        assert_eq!(c.arms.length, 4.0);
        assert_eq!(c.arms.gap, 2.0);
        assert_eq!(c.opacity, 100.0);
    }

    #[test]
    fn valorant_dot_only_and_custom_color() {
        let (_, c) = decode("0;s;1;P;c;8;u;FC5A8DFF;h;0;d;1;z;3;0b;0;1b;0;S;c;0")
            .unwrap()
            .unwrap();
        assert_eq!(c.color, "#FC5A8D");
        assert!(!c.arms.enabled);
        assert!(c.dot.enabled);
        assert_eq!(c.dot.size, 3.0);
    }

    #[test]
    fn cs2_legacy_v1_round_trips_through_base57() {
        let mut bytes = [0u8; 18];
        bytes[1] = 1;
        bytes[2] = (-20i8) as u8; // gap -2
        bytes[3] = 2; // contorno 1
        bytes[10] = 1 | 8; // verde, con contorno
        bytes[12] = 10; // grosor 1
        bytes[13] = (1 | 8) << 4; // punto y estilo T
        bytes[14] = 30; // largo 3
        let code = encode_legacy(bytes);
        let (source, c) = decode(&code).unwrap().unwrap();
        assert_eq!(source, GameCodeSource::Cs2);
        assert_eq!(c.color, "#32FA32");
        assert!(c.outline.enabled);
        assert_eq!(c.arms.gap, 2.0);
        assert_eq!(c.arms.length, 6.0);
        assert_eq!(c.arms.thickness, 2.0);
        assert!(!c.arms.top && c.arms.bottom);
        assert!(c.dot.enabled);
    }

    #[test]
    fn cs2_pixel_v3_scales_to_1080() {
        let mut bytes = [0u8; 18];
        bytes[1] = 3;
        bytes[2] = 0x20; // contorno
        bytes[3] = 255;
        bytes[6] = 255;
        bytes[7] = 4; // separación
        bytes[8] = 8; // largo
        let bits: u32 = 2 << 23; // grosor 2
        bytes[10..14].copy_from_slice(&bits.to_le_bytes());
        bytes[14..16].copy_from_slice(&1440u16.to_le_bytes());
        let (_, c) = decode(&encode_legacy(bytes)).unwrap().unwrap();
        assert_eq!(c.color, "#FF0000");
        assert_eq!(c.arms.length, 6.0); // 8 px a 1440 → 6 a 1080
        assert_eq!(c.arms.gap, 3.0);
    }

    #[test]
    fn a_typo_fails_the_checksum() {
        let mut bytes = [0u8; 18];
        bytes[1] = 1;
        bytes[14] = 30;
        let code = encode_legacy(bytes);
        let broken: String = code
            .chars()
            .enumerate()
            .map(|(i, c)| {
                if i == 6 {
                    if c == 'A' { 'B' } else { 'A' }
                } else {
                    c
                }
            })
            .collect();
        assert_eq!(decode(&broken), Some(Err(GameCodeError::BadChecksum)));
    }

    #[test]
    fn other_text_is_not_a_game_code() {
        assert_eq!(decode("VRC1-01T3X-X54"), None);
        assert_eq!(decode("hola"), None);
    }
}
