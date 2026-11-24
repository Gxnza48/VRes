//! `vres-cli zoom test` y `vres-cli input probe`.
//!
//! El zoom se verifica por lectura (`MagGetFullscreenTransform`) y por
//! captura: dos cuadrados de color, uno en el centro de la pantalla y otro
//! corrido; con zoom, el del centro tiene que quedar en su lugar y el corrido
//! tiene que aparecer `aumento` veces más lejos. Al terminar (o si el proceso
//! muere) el zoom desaparece: es de esta sesión de la Magnification API.

use std::time::{Duration, Instant};

use serde_json::{Value, json};
use vres_core::enhancers::{MouseButton, ZOOM_WARMUP_LEVEL, zoom_offset};
use vres_core::model::display::DisplayInfo;
use vres_win::keyboard;
use vres_win::mag::Magnifier;
use vres_win::probe::{self, ColorPatch};

use crate::Report;
use crate::hw::list_displays;

const CENTER_COLOR: [u8; 3] = [200, 60, 170];
const SIDE_COLOR: [u8; 3] = [60, 190, 80];
/// Distancia del cuadrado corrido al centro, sin zoom.
const SIDE_DISTANCE: i32 = 80;
const PATCH_SIZE: i32 = 40;
const SETTLE: Duration = Duration::from_millis(350);

fn close(a: [u8; 3], b: [u8; 3]) -> bool {
    (0..3).all(|i| a[i].abs_diff(b[i]) <= 12)
}

fn zoom_display(magnifier: &Magnifier, display: &DisplayInfo, level: f32, hold: Duration) -> Value {
    let (cx, cy) = display.bounds.center();
    let seen_at = (SIDE_DISTANCE as f32 * level).round() as i32;
    let result = (|| -> vres_win::WinResult<Value> {
        let center = ColorPatch::show(cx, cy, PATCH_SIZE, CENTER_COLOR)?;
        let side = ColorPatch::show(
            cx + SIDE_DISTANCE,
            cy + SIDE_DISTANCE,
            PATCH_SIZE,
            SIDE_COLOR,
        )?;
        probe::pump_for(SETTLE);
        // Se compara contra lo capturado antes del zoom: si hay un filtro de
        // color activo, los colores capturados no son los puros.
        let before_center = probe::read_screen_pixel(cx, cy)?;
        let before_side = probe::read_screen_pixel(cx + SIDE_DISTANCE, cy + SIDE_DISTANCE)?;
        let before_far = probe::read_screen_pixel(cx + seen_at, cy + seen_at)?;

        let (x, y) = zoom_offset(level, (cx, cy));
        let started = Instant::now();
        magnifier.set_transform(level, x, y)?;
        let write_ms = started.elapsed().as_secs_f64() * 1000.0;
        let readback = magnifier.transform()?;
        probe::pump_for(SETTLE.max(hold));
        let zoomed_center = probe::read_screen_pixel(cx, cy)?;
        let zoomed_far = probe::read_screen_pixel(cx + seen_at, cy + seen_at)?;

        magnifier.set_transform(1.0, 0, 0)?;
        let cleared = magnifier.transform()?;
        probe::pump_for(SETTLE);
        let after_far = probe::read_screen_pixel(cx + seen_at, cy + seen_at)?;
        drop((center, side));

        let readback_ok = (readback.0 - level).abs() < 1e-3 && readback.1 == x && readback.2 == y;
        let center_kept = close(zoomed_center, before_center);
        let mapped = close(zoomed_far, before_side) && !close(before_far, before_side);
        let restored = (cleared.0 - 1.0).abs() < 1e-3 && close(after_far, before_far);
        Ok(json!({
            "ok": readback_ok && restored,
            "display": display.id,
            "gdiName": display.gdi_name,
            "center": [cx, cy],
            "offset": [x, y],
            "writeMs": (write_ms * 100.0).round() / 100.0,
            "readbackOk": readback_ok,
            "capture": {
                "centerBefore": before_center,
                "centerZoomed": zoomed_center,
                "sideBefore": before_side,
                "farBefore": before_far,
                "farZoomed": zoomed_far,
                "centerKept": center_kept,
                "sideMapped": mapped,
                "visibleInCapture": center_kept && mapped,
            },
            "restored": restored,
        }))
    })();
    result.unwrap_or_else(
        |error| json!({ "ok": false, "display": display.id, "error": error.to_string() }),
    )
}

/// Zoom sobre el centro de cada pantalla (o de una), y vuelta atrás.
pub fn test(display: Option<&str>, level: f32, hold: Duration) -> Report {
    let displays = match list_displays() {
        Ok(displays) => displays,
        Err(report) => return report,
    };
    let level = vres_core::model::ranges::ZOOM.clamp(level);
    let targets: Vec<&DisplayInfo> = displays
        .iter()
        .filter(|candidate| {
            display.is_none_or(|wanted| candidate.id == wanted || candidate.gdi_name == wanted)
        })
        .collect();
    if targets.is_empty() {
        return Report::fail(
            json!({ "ok": false, "error": "display not found" }),
            "No se encontró esa pantalla.",
        );
    }
    let magnifier = match Magnifier::init() {
        Ok(magnifier) => magnifier,
        Err(error) => {
            return Report::fail(
                json!({ "ok": false, "error": error.to_string() }),
                format!("La Magnification API no está disponible: {error}"),
            );
        }
    };
    // La primera llamada puede tardar (Windows prepara el zoom): se mide
    // aparte, con aumento 1 (no cambia nada en pantalla).
    let started = Instant::now();
    let warmup = magnifier.set_transform(ZOOM_WARMUP_LEVEL, 0, 0).is_ok()
        && magnifier.set_transform(1.0, 0, 0).is_ok();
    let warmup_ms = (started.elapsed().as_secs_f64() * 100_000.0).round() / 100.0;
    let results: Vec<Value> = targets
        .iter()
        .map(|target| zoom_display(&magnifier, target, level, hold))
        .collect();
    drop(magnifier);
    let ok = results.iter().all(|result| result["ok"] == true);
    let value = json!({
        "ok": ok,
        "level": level,
        "warmup": { "ok": warmup, "ms": warmup_ms },
        "displays": results,
    });
    let text = serde_json::to_string_pretty(&value).unwrap_or_default();
    if ok {
        Report::ok(value, text)
    } else {
        Report::fail(value, text)
    }
}

/// Anota los botones del mouse que se aprietan durante `duration` (lo mismo
/// que lee el motor para Dark Boost y Zoom: solo lectura, sin hooks).
pub fn input_probe(duration: Duration) -> Report {
    const BUTTONS: [(MouseButton, &str); 4] = [
        (MouseButton::Right, "right"),
        (MouseButton::Middle, "middle"),
        (MouseButton::Back, "back"),
        (MouseButton::Forward, "forward"),
    ];
    let start = Instant::now();
    let mut down = [false; 4];
    let mut events = Vec::new();
    let mut polls = 0u64;
    while start.elapsed() < duration {
        polls += 1;
        for (index, (button, name)) in BUTTONS.iter().enumerate() {
            let now = keyboard::is_down(button.virtual_key());
            if now != down[index] {
                down[index] = now;
                events.push(json!({
                    "ms": start.elapsed().as_millis() as u64,
                    "button": name,
                    "down": now,
                }));
            }
        }
        std::thread::sleep(Duration::from_millis(15));
    }
    let text = format!("{} lecturas, {} cambios", polls, events.len());
    Report::ok(
        json!({ "ok": true, "polls": polls, "events": events }),
        text,
    )
}

/// `vres-cli probe engine-enhancers`: el motor real empieza a escuchar las
/// teclas al armar Dark Boost y Zoom (en "siempre") y deja de escucharlas al
/// desarmarlos. Nadie aprieta nada: no cambia la pantalla.
pub fn probe_engine(hold: Duration) -> Report {
    use std::sync::{Arc, Mutex, mpsc};
    use vres_core::enhancers::{EnhancerScope, EnhancerSettings};
    use vres_core::ipc::EnhancerState;
    use vres_core::planner::Requested;
    use vres_engine::{Command, EngineConfig, EngineEvent, EngineSettings};

    let states: Arc<Mutex<Vec<EnhancerState>>> = Arc::default();
    let seen = Arc::clone(&states);
    let sink: vres_engine::EventSink = Arc::new(move |event| {
        if let EngineEvent::Enhancers(state) = event
            && let Ok(mut list) = seen.lock()
        {
            list.push(state);
        }
    });
    let last = || {
        states
            .lock()
            .ok()
            .and_then(|list| list.last().copied())
            .unwrap_or_default()
    };

    let mut enhancers = EnhancerSettings::default();
    enhancers.dark_boost.armed = true;
    enhancers.dark_boost.scope = EnhancerScope::Always;
    enhancers.zoom.armed = true;
    enhancers.zoom.scope = EnhancerScope::Always;
    let config = EngineConfig {
        restore_path: None,
        modes_restore_path: None,
        requested: Requested::default(),
        settings: EngineSettings {
            enhancers,
            ..EngineSettings::default()
        },
    };
    let started = Instant::now();
    let (mut engine, _snapshot) = match vres_engine::spawn(config, sink) {
        Ok(started) => started,
        Err(error) => {
            return Report::fail(
                json!({ "ok": false, "error": error }),
                "El motor no pudo arrancar.",
            );
        }
    };
    let startup_ms = started.elapsed().as_millis() as u64;
    std::thread::sleep(hold);
    let armed = last();

    let (tx, rx) = mpsc::channel();
    let sent = engine.send(Command::Configure {
        settings: Box::new(EngineSettings::default()),
        reply: tx,
    });
    let configured = sent && rx.recv_timeout(Duration::from_secs(3)).is_ok();
    std::thread::sleep(Duration::from_millis(200));
    let disarmed = last();
    let stopped = engine.shutdown(Duration::from_secs(6));
    let transform = Magnifier::init()
        .and_then(|magnifier| magnifier.transform())
        .ok();
    let zoom_clear =
        transform.is_none_or(|(level, x, y)| (level - 1.0).abs() < 1e-3 && x == 0 && y == 0);

    let ok = armed.dark_boost_listening
        && armed.zoom_listening
        && !armed.dark_boost
        && !armed.zoom
        && configured
        && !disarmed.dark_boost_listening
        && !disarmed.zoom_listening
        && stopped
        && zoom_clear;
    let value = json!({
        "ok": ok,
        "startupMs": startup_ms,
        "armed": armed,
        "configured": configured,
        "disarmed": disarmed,
        "engineStopped": stopped,
        "zoomClear": zoom_clear,
    });
    let text = serde_json::to_string_pretty(&value).unwrap_or_default();
    if ok {
        Report::ok(value, text)
    } else {
        Report::fail(value, text)
    }
}
