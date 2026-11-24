//! Ícono y menú de la bandeja del sistema.
//!
//! La interfaz puede no existir (VRes vive en la bandeja con la ventana
//! destruida), así que el menú nativo tiene que alcanzar para lo esencial.

use tauri::AppHandle;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::app_windows;
use crate::i18n::{Lang, tray_texts};

const OPEN: &str = "open";
const PAUSE: &str = "pause";
const RESET: &str = "reset";
const QUIT: &str = "quit";

/// Crea el ícono de la bandeja con su menú.
pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let texts = tray_texts(Lang::system());

    let open = MenuItem::with_id(app, OPEN, texts.open, true, None::<&str>)?;
    // Pausar y restablecer se habilitan cuando exista el motor de color (F1).
    let pause = MenuItem::with_id(app, PAUSE, texts.pause, false, None::<&str>)?;
    let reset = MenuItem::with_id(app, RESET, texts.reset, false, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, QUIT, texts.quit, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &pause, &reset, &separator, &quit])?;

    let mut builder = TrayIconBuilder::with_id("vres")
        .tooltip(texts.tooltip)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(on_menu_event)
        .on_tray_icon_event(on_tray_icon_event);
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        OPEN => app_windows::open_main_async(app),
        QUIT => app.exit(0),
        _ => {}
    }
}

fn on_tray_icon_event(tray: &TrayIcon, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        app_windows::open_main_async(tray.app_handle());
    }
}
