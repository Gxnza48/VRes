//! Traducción de atajos (tecla física + modificadores) a lo que pide
//! `RegisterHotKey`.

use vres_core::enhancers::Binding;
use vres_core::keys::{Hotkey, KeySpec, hold_key_spec, key_spec};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, MAPVK_VSC_TO_VK_EX, MapVirtualKeyW,
};

const MOD_ALT: u32 = 0x1;
const MOD_CONTROL: u32 = 0x2;
const MOD_SHIFT: u32 = 0x4;
const MOD_WIN: u32 = 0x8;
const MOD_NOREPEAT: u32 = 0x4000;

/// Tecla virtual de una tecla física, según la distribución de teclado activa.
pub fn virtual_key(spec: KeySpec) -> Option<u32> {
    match spec {
        KeySpec::Vk(vk) => Some(u32::from(vk)),
        KeySpec::Scan(scan) => {
            // SAFETY: función pura de traducción; un código desconocido devuelve 0.
            let vk = unsafe { MapVirtualKeyW(u32::from(scan), MAPVK_VSC_TO_VK_EX) };
            (vk != 0).then_some(vk)
        }
    }
}

/// Tecla virtual de lo que hay que apretar para Dark Boost o Zoom.
pub fn binding_virtual_key(binding: &Binding) -> Option<u32> {
    match binding {
        Binding::Key { code } => virtual_key(hold_key_spec(code)?),
        Binding::Mouse { button } => Some(button.virtual_key()),
    }
}

/// `true` si la tecla (o el botón del mouse) está apretada ahora. Solo lee:
/// no la consume, así que el programa al frente la recibe igual.
pub fn is_down(vk: u32) -> bool {
    let Ok(vk) = i32::try_from(vk) else {
        return false;
    };
    // SAFETY: solo consulta el estado de una tecla virtual.
    let state = unsafe { GetAsyncKeyState(vk) };
    (state as u16) & 0x8000 != 0
}

/// `(modificadores, tecla virtual)` para `RegisterHotKey`. `None` si la tecla
/// no se puede registrar.
pub fn hotkey_registration(hotkey: &Hotkey) -> Option<(u32, u32)> {
    let vk = virtual_key(key_spec(&hotkey.code)?)?;
    let mut modifiers = MOD_NOREPEAT;
    if hotkey.alt {
        modifiers |= MOD_ALT;
    }
    if hotkey.ctrl {
        modifiers |= MOD_CONTROL;
    }
    if hotkey.shift {
        modifiers |= MOD_SHIFT;
    }
    if hotkey.win {
        modifiers |= MOD_WIN;
    }
    Some((modifiers, vk))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_keys_translate_without_the_layout() {
        let hotkey = Hotkey {
            code: "F8".into(),
            ctrl: true,
            alt: false,
            shift: true,
            win: false,
        };
        assert_eq!(
            hotkey_registration(&hotkey),
            Some((MOD_NOREPEAT | MOD_CONTROL | MOD_SHIFT, 0x77))
        );
    }

    #[test]
    fn letters_translate_through_the_layout() {
        let hotkey = Hotkey {
            code: "KeyR".into(),
            ctrl: true,
            alt: true,
            shift: false,
            win: false,
        };
        let (_, vk) = hotkey_registration(&hotkey).expect("traducible");
        // En cualquier distribución latina la tecla física R es una letra.
        assert!((0x41..=0x5A).contains(&vk), "vk inesperada: {vk:#x}");
    }
}
