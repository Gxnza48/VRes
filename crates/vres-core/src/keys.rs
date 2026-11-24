//! Atajos de teclado: teclas físicas y sus choques conocidos.
//!
//! Los atajos se guardan como **tecla física** (`KeyboardEvent.code`), no como
//! carácter: así `Ctrl + Alt + Q` es la misma tecla en un teclado español,
//! latinoamericano, ABNT2 o inglés. Al registrarlo, `vres-win` la traduce a la
//! tecla virtual de la distribución activa.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Cómo se traduce una tecla física a Windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeySpec {
    /// Código de escaneo (set 1): la tecla virtual depende de la distribución.
    Scan(u16),
    /// Tecla virtual fija (F1, flechas, teclado numérico…).
    Vk(u8),
}

/// Un atajo: una tecla física y sus modificadores.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Hotkey {
    /// `KeyboardEvent.code`, por ejemplo `KeyR`, `F8` o `Numpad5`.
    pub code: String,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
}

/// Problemas de un atajo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum HotkeyIssue {
    /// Tecla desconocida o no admitida.
    UnknownKey,
    /// Letras, números, signos y espacio necesitan Ctrl, Alt o Win.
    NeedsModifier,
    /// Reservado: el atajo de emergencia, F12 (Windows) o Escape.
    Reserved,
    /// Ctrl + Alt es AltGr en teclados latinoamericanos y ABNT2: el atajo se
    /// comería caracteres como la arroba.
    AltGrClash,
    /// Lo usa el overlay de la placa de video (por ejemplo Alt + Z).
    OverlayClash,
    /// Otro programa ya registró ese atajo.
    Taken,
}

/// Resultado de revisar un atajo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HotkeyCheck {
    /// Impide guardarlo.
    pub error: Option<HotkeyIssue>,
    /// Se puede guardar, pero conviene otro.
    pub warning: Option<HotkeyIssue>,
}

/// Tecla física → cómo registrarla en Windows. `None` si no se admite.
pub fn key_spec(code: &str) -> Option<KeySpec> {
    use KeySpec::{Scan, Vk};

    if let Some(letter) = code.strip_prefix("Key") {
        let scan = match letter {
            "A" => 0x1E,
            "B" => 0x30,
            "C" => 0x2E,
            "D" => 0x20,
            "E" => 0x12,
            "F" => 0x21,
            "G" => 0x22,
            "H" => 0x23,
            "I" => 0x17,
            "J" => 0x24,
            "K" => 0x25,
            "L" => 0x26,
            "M" => 0x32,
            "N" => 0x31,
            "O" => 0x18,
            "P" => 0x19,
            "Q" => 0x10,
            "R" => 0x13,
            "S" => 0x1F,
            "T" => 0x14,
            "U" => 0x16,
            "V" => 0x2F,
            "W" => 0x11,
            "X" => 0x2D,
            "Y" => 0x15,
            "Z" => 0x2C,
            _ => return None,
        };
        return Some(Scan(scan));
    }
    if let Some(digit) = code.strip_prefix("Digit") {
        let value: u16 = digit.parse().ok().filter(|d| *d <= 9)?;
        // 1 → 0x02 … 9 → 0x0A, 0 → 0x0B.
        return Some(Scan(if value == 0 { 0x0B } else { value + 1 }));
    }
    if let Some(number) = code.strip_prefix('F')
        && let Ok(n) = number.parse::<u8>()
    {
        return (1..=24).contains(&n).then_some(Vk(0x6F + n));
    }
    if let Some(rest) = code.strip_prefix("Numpad") {
        if let Ok(d) = rest.parse::<u8>() {
            return (d <= 9).then_some(Vk(0x60 + d));
        }
        return match rest {
            "Multiply" => Some(Vk(0x6A)),
            "Add" => Some(Vk(0x6B)),
            "Subtract" => Some(Vk(0x6D)),
            "Decimal" => Some(Vk(0x6E)),
            "Divide" => Some(Vk(0x6F)),
            _ => None,
        };
    }
    Some(match code {
        "Minus" => Scan(0x0C),
        "Equal" => Scan(0x0D),
        "BracketLeft" => Scan(0x1A),
        "BracketRight" => Scan(0x1B),
        "Semicolon" => Scan(0x27),
        "Quote" => Scan(0x28),
        "Backquote" => Scan(0x29),
        "Backslash" => Scan(0x2B),
        "Comma" => Scan(0x33),
        "Period" => Scan(0x34),
        "Slash" => Scan(0x35),
        "IntlBackslash" => Scan(0x56),
        "Space" => Vk(0x20),
        "Tab" => Vk(0x09),
        "Enter" => Vk(0x0D),
        "Backspace" => Vk(0x08),
        "Escape" => Vk(0x1B),
        "Insert" => Vk(0x2D),
        "Delete" => Vk(0x2E),
        "Home" => Vk(0x24),
        "End" => Vk(0x23),
        "PageUp" => Vk(0x21),
        "PageDown" => Vk(0x22),
        "ArrowUp" => Vk(0x26),
        "ArrowDown" => Vk(0x28),
        "ArrowLeft" => Vk(0x25),
        "ArrowRight" => Vk(0x27),
        "Pause" => Vk(0x13),
        "ScrollLock" => Vk(0x91),
        _ => return None,
    })
}

/// Teclas que escriben algo: sin modificador, el atajo impediría escribirlas.
fn is_typing_key(code: &str) -> bool {
    matches!(key_spec(code), Some(KeySpec::Scan(_)))
        || matches!(code, "Space" | "Tab" | "Enter" | "Backspace")
}

/// Revisa un atajo antes de guardarlo.
pub fn check_hotkey(hotkey: &Hotkey) -> HotkeyCheck {
    let error = if key_spec(&hotkey.code).is_none() {
        Some(HotkeyIssue::UnknownKey)
    } else if hotkey.code == "Escape"
        || hotkey.code == "F12"
        || (hotkey.code == "KeyR" && hotkey.ctrl && hotkey.alt && hotkey.shift && !hotkey.win)
    {
        Some(HotkeyIssue::Reserved)
    } else if is_typing_key(&hotkey.code) && !(hotkey.ctrl || hotkey.alt || hotkey.win) {
        Some(HotkeyIssue::NeedsModifier)
    } else {
        None
    };

    let only_alt = hotkey.alt && !hotkey.ctrl && !hotkey.shift && !hotkey.win;
    let warning = if error.is_some() {
        None
    } else if hotkey.ctrl && hotkey.alt && matches!(key_spec(&hotkey.code), Some(KeySpec::Scan(_)))
    {
        Some(HotkeyIssue::AltGrClash)
    } else if only_alt && matches!(hotkey.code.as_str(), "KeyZ" | "F1" | "F9" | "F10") {
        Some(HotkeyIssue::OverlayClash)
    } else {
        None
    };
    HotkeyCheck { error, warning }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: &str, ctrl: bool, alt: bool, shift: bool) -> Hotkey {
        Hotkey {
            code: code.into(),
            ctrl,
            alt,
            shift,
            win: false,
        }
    }

    #[test]
    fn letters_and_digits_use_scan_codes() {
        assert_eq!(key_spec("KeyR"), Some(KeySpec::Scan(0x13)));
        assert_eq!(key_spec("KeyQ"), Some(KeySpec::Scan(0x10)));
        assert_eq!(key_spec("Digit1"), Some(KeySpec::Scan(0x02)));
        assert_eq!(key_spec("Digit0"), Some(KeySpec::Scan(0x0B)));
        assert_eq!(key_spec("Digit10"), None);
    }

    #[test]
    fn function_and_numpad_keys_are_fixed() {
        assert_eq!(key_spec("F1"), Some(KeySpec::Vk(0x70)));
        assert_eq!(key_spec("F8"), Some(KeySpec::Vk(0x77)));
        assert_eq!(key_spec("F24"), Some(KeySpec::Vk(0x87)));
        assert_eq!(key_spec("F25"), None);
        assert_eq!(key_spec("Numpad5"), Some(KeySpec::Vk(0x65)));
        assert_eq!(key_spec("NumpadAdd"), Some(KeySpec::Vk(0x6B)));
        assert_eq!(key_spec("NumpadEnter"), None);
        assert_eq!(key_spec("Fn"), None);
        assert_eq!(key_spec("MetaLeft"), None);
    }

    #[test]
    fn reserved_and_unknown_keys_are_errors() {
        let emergency = key("KeyR", true, true, true);
        assert_eq!(check_hotkey(&emergency).error, Some(HotkeyIssue::Reserved));
        assert_eq!(
            check_hotkey(&key("F12", false, false, false)).error,
            Some(HotkeyIssue::Reserved)
        );
        assert_eq!(
            check_hotkey(&key("Escape", true, false, false)).error,
            Some(HotkeyIssue::Reserved)
        );
        assert_eq!(
            check_hotkey(&key("ContextMenu", true, false, false)).error,
            Some(HotkeyIssue::UnknownKey)
        );
    }

    #[test]
    fn typing_keys_need_a_modifier() {
        assert_eq!(
            check_hotkey(&key("KeyP", false, false, true)).error,
            Some(HotkeyIssue::NeedsModifier),
            "Shift solo no alcanza"
        );
        assert_eq!(check_hotkey(&key("KeyP", true, false, false)).error, None);
        assert_eq!(
            check_hotkey(&key("F8", false, false, false)),
            HotkeyCheck::default()
        );
        assert_eq!(
            check_hotkey(&key("Numpad1", false, false, false)).error,
            None
        );
    }

    #[test]
    fn known_clashes_are_warnings() {
        let altgr = check_hotkey(&key("KeyQ", true, true, false));
        assert_eq!(altgr.error, None);
        assert_eq!(altgr.warning, Some(HotkeyIssue::AltGrClash));
        assert_eq!(
            check_hotkey(&key("KeyZ", false, true, false)).warning,
            Some(HotkeyIssue::OverlayClash)
        );
        assert_eq!(check_hotkey(&key("F8", true, true, false)).warning, None);
    }
}
