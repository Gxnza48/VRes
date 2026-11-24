//! `vres-cli overlay test`: dibuja un crosshair de prueba (un píxel magenta)
//! en el centro de cada pantalla, lo lee de la pantalla para verificar el
//! centrado y prueba que "ocultar en capturas" lo saque de la captura.

use std::time::Duration;

use serde_json::json;
use vres_core::crosshair::{Arms, Crosshair, Dot, Outline, render};
use vres_core::model::display::DisplayInfo;
use vres_win::overlay::CrosshairOverlay;
use vres_win::{displays, probe};

use crate::Report;

const MAGENTA: [u8; 3] = [255, 0, 255];

fn failure(error: impl std::fmt::Display, text: &str) -> Report {
    Report::fail(json!({ "ok": false, "error": error.to_string() }), text)
}

/// `vres-cli overlay test`.
pub fn overlay_test(selector: Option<&str>, hold: Duration) -> Report {
    let found = match displays::enumerate() {
        Ok(found) => found,
        Err(error) => return failure(error, "No se pudieron enumerar las pantallas."),
    };
    let chosen: Vec<DisplayInfo> = match selector {
        None => found.clone(),
        Some(selector) => {
            let display = match selector.parse::<usize>() {
                Ok(number) => found.get(number.wrapping_sub(1)),
                Err(_) => found.iter().find(|display| {
                    display.gdi_name.eq_ignore_ascii_case(selector) || display.id == selector
                }),
            };
            match display {
                Some(display) => vec![display.clone()],
                None => {
                    return failure(
                        format!("display not found: {selector}"),
                        "No existe esa pantalla. Mira la lista con `vres-cli monitors`.",
                    );
                }
            }
        }
    };

    let crosshair = Crosshair {
        color: "#FF00FF".into(),
        arms: Arms {
            enabled: false,
            ..Arms::default()
        },
        dot: Dot {
            enabled: true,
            size: 1.0,
            round: false,
        },
        outline: Outline {
            enabled: false,
            ..Outline::default()
        },
        ..Crosshair::default()
    };
    let rendered = match render(&crosshair) {
        Ok(rendered) => rendered,
        Err(error) => return failure(error, "No se pudo dibujar el crosshair de prueba."),
    };
    let bgra = rendered.to_bgra_premultiplied();

    let mut ok = true;
    let mut results = Vec::new();
    let mut text = String::new();
    for display in &chosen {
        let bounds = display.bounds;
        let center_x = bounds.x + (bounds.width / 2) as i32;
        let center_y = bounds.y + (bounds.height / 2) as i32;
        let mut overlay = match CrosshairOverlay::create() {
            Ok(overlay) => overlay,
            Err(error) => return failure(error, "No se pudo crear la ventana del crosshair."),
        };
        let shown = overlay.show(
            &bgra,
            rendered.size,
            center_x - rendered.center as i32,
            center_y - rendered.center as i32,
        );
        probe::pump_for(Duration::from_millis(400));
        let at_center = probe::read_screen_pixel(center_x, center_y).ok();
        let beside = probe::read_screen_pixel(center_x + 1, center_y).ok();
        let centered = at_center == Some(MAGENTA) && beside != Some(MAGENTA);

        let capture_api = overlay.set_hidden_from_capture(true);
        probe::pump_for(Duration::from_millis(300));
        let hidden = probe::read_screen_pixel(center_x, center_y).ok();
        let excluded = capture_api && hidden != Some(MAGENTA);
        probe::pump_for(hold);
        drop(overlay);

        ok &= shown.is_ok() && centered;
        results.push(json!({
            "name": display.friendly_name,
            "center": { "x": center_x, "y": center_y },
            "shown": shown.is_ok(),
            "pixel": at_center,
            "besidePixel": beside,
            "centered": centered,
            "hiddenFromCapture": excluded,
        }));
        text.push_str(&format!(
            "{}: centro ({center_x}, {center_y}) {} · ocultar en capturas {}\n",
            display.friendly_name,
            if centered {
                "exacto ✓"
            } else {
                "NO coincide ✗"
            },
            if excluded { "✓" } else { "no disponible" },
        ));
    }
    let value = json!({ "ok": ok, "displays": results });
    if ok {
        Report::ok(value, text.trim_end().to_owned())
    } else {
        Report::fail(value, text.trim_end().to_owned())
    }
}
