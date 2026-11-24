//! `vres-cli vibrance`: la vibrance del driver en hardware real.
//!
//! `set` aplica un valor (escala de VRes: 0–100, 50 = normal), lo relee, lo
//! mantiene y vuelve al original, aunque la prueba falle o se corte.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use vres_core::model::display::DisplayInfo;
use vres_win::displays;
use vres_win::vibrance::DriverVibrance;

use crate::Report;

/// Originales pendientes de restaurar ante `Ctrl + C` (gdi, nivel del driver).
static PENDING: Mutex<Vec<(DisplayInfo, i32)>> = Mutex::new(Vec::new());

/// Restaura lo pendiente (lo llama el manejador de `Ctrl + C`).
pub fn restore_pending() {
    if let Ok(pending) = PENDING.lock() {
        let mut driver = DriverVibrance::load();
        for (display, raw) in pending.iter() {
            let _ = driver.set(display, *raw);
        }
    }
}

fn failure(error: impl std::fmt::Display, text: &str) -> Report {
    Report::fail(json!({ "ok": false, "error": error.to_string() }), text)
}

fn pick(found: &[DisplayInfo], selector: Option<&str>) -> Result<Vec<DisplayInfo>, Report> {
    let Some(selector) = selector else {
        return Ok(found.to_vec());
    };
    let display = match selector.parse::<usize>() {
        Ok(number) => found.get(number.wrapping_sub(1)),
        Err(_) => found.iter().find(|display| {
            display.gdi_name.eq_ignore_ascii_case(selector) || display.id == selector
        }),
    };
    display
        .cloned()
        .map(|display| vec![display])
        .ok_or_else(|| {
            failure(
                format!("display not found: {selector}"),
                "No existe esa pantalla. Mira la lista con `vres-cli monitors`.",
            )
        })
}

/// `vres-cli vibrance get`.
pub fn get(selector: Option<&str>) -> Report {
    let found = match displays::enumerate() {
        Ok(found) => found,
        Err(error) => return failure(error, "No se pudieron enumerar las pantallas."),
    };
    let chosen = match pick(&found, selector) {
        Ok(chosen) => chosen,
        Err(report) => return report,
    };
    let mut driver = DriverVibrance::load();
    let (nvidia, amd) = driver.loaded();
    let mut list = Vec::new();
    let mut text = format!(
        "Drivers: NVAPI {} · ADL {}\n",
        if nvidia { "sí" } else { "no" },
        if amd { "sí" } else { "no" }
    );
    for display in &chosen {
        let info = driver.info(display);
        list.push(json!({
            "id": display.id,
            "name": display.friendly_name,
            "vendor": display.adapter.vendor,
            "backend": info.map(|i| i.backend),
            "current": info.map(|i| i.current),
            "min": info.map(|i| i.min),
            "max": info.map(|i| i.max),
            "default": info.map(|i| i.default),
            "value": info.map(|i| (i.value_for(i.current) * 10.0).round() / 10.0),
        }));
        text.push_str(&match info {
            Some(info) => format!(
                "{}: {:?} {} (rango {}–{}, normal {}) = {:.0} en la escala de VRes\n",
                display.friendly_name,
                info.backend,
                info.current,
                info.min,
                info.max,
                info.default,
                info.value_for(info.current)
            ),
            None => format!("{}: sin vibrance del driver\n", display.friendly_name),
        });
    }
    Report::ok(
        json!({ "ok": true, "nvapi": nvidia, "adl": amd, "displays": list }),
        text.trim_end().to_owned(),
    )
}

/// `vres-cli probe engine-vibrance`: el motor real aplica una vibrance a
/// todas las pantallas con driver, la verifica y, al apagarse, devuelve los
/// valores originales. Sin tocar los archivos de estado de la app instalada.
pub fn probe_engine(value: f32, hold: Duration) -> Report {
    use std::sync::Arc;
    use vres_core::model::look::Look;
    use vres_core::planner::Requested;
    use vres_engine::{EngineConfig, EngineSettings};

    let found = match displays::enumerate() {
        Ok(found) => found,
        Err(error) => return failure(error, "No se pudieron enumerar las pantallas."),
    };
    let mut driver = DriverVibrance::load();
    let before: Vec<(DisplayInfo, vres_core::model::vibrance::VibranceInfo)> = found
        .iter()
        .filter_map(|display| driver.info(display).map(|info| (display.clone(), info)))
        .collect();
    if before.is_empty() {
        return failure(
            "no vibrance backend",
            "Ninguna pantalla tiene vibrance del driver.",
        );
    }
    if let Ok(mut pending) = PENDING.lock() {
        pending.extend(
            before
                .iter()
                .map(|(display, info)| (display.clone(), info.current)),
        );
    }

    let sink: vres_engine::EventSink = Arc::new(|_| {});
    let config = EngineConfig {
        restore_path: None,
        modes_restore_path: None,
        requested: Requested {
            look: Look {
                vibrance: value,
                ..Look::default()
            },
            ..Requested::default()
        },
        settings: EngineSettings::default(),
    };
    let (mut engine, snapshot) = match vres_engine::spawn(config, sink) {
        Ok(started) => started,
        Err(error) => return failure(error, "El motor no pudo arrancar."),
    };
    std::thread::sleep(hold);
    let during: Vec<Option<i32>> = before
        .iter()
        .map(|(display, _)| driver.info(display).map(|info| info.current))
        .collect();
    let stopped = engine.shutdown(Duration::from_secs(6));
    let after: Vec<Option<i32>> = before
        .iter()
        .map(|(display, _)| driver.info(display).map(|info| info.current))
        .collect();
    if let Ok(mut pending) = PENDING.lock() {
        pending.clear();
    }

    let mut ok = stopped;
    let mut list = Vec::new();
    for (((display, info), during), after) in before.iter().zip(&during).zip(&after) {
        let expected = info.raw_for(value);
        let applied = *during == Some(expected);
        let restored = *after == Some(info.current);
        ok &= applied && restored;
        list.push(json!({
            "name": display.friendly_name,
            "backend": info.backend,
            "original": info.current,
            "expected": expected,
            "during": during,
            "after": after,
            "applied": applied,
            "restored": restored,
        }));
    }
    let reported = snapshot.status.vibrance.len();
    let value_json = json!({ "ok": ok, "engineStopped": stopped, "reportedBackends": reported, "displays": list });
    if ok {
        Report::ok(
            value_json,
            "El motor aplica la vibrance en cada driver y la devuelve al apagarse.",
        )
    } else {
        Report::fail(
            value_json,
            "La vibrance del motor no terminó bien: mira el JSON.",
        )
    }
}

/// `vres-cli vibrance set`: aplica, relee, mantiene y restaura.
pub fn set(selector: &str, value: f32, hold: Duration) -> Report {
    let found = match displays::enumerate() {
        Ok(found) => found,
        Err(error) => return failure(error, "No se pudieron enumerar las pantallas."),
    };
    let display = match pick(&found, Some(selector)) {
        Ok(mut chosen) => chosen.remove(0),
        Err(report) => return report,
    };
    let mut driver = DriverVibrance::load();
    let Some(before) = driver.info(&display) else {
        return failure(
            "no vibrance backend",
            "Esa pantalla no tiene vibrance del driver.",
        );
    };
    if let Ok(mut pending) = PENDING.lock() {
        pending.push((display.clone(), before.current));
    }

    let raw = before.raw_for(value);
    let started = Instant::now();
    let written = driver.set(&display, raw);
    let write_ms = started.elapsed().as_secs_f64() * 1000.0;
    let during = driver.info(&display);
    if written.is_ok() {
        std::thread::sleep(hold);
    }
    let restored = driver.set(&display, before.current);
    let after = driver.info(&display);
    if let Ok(mut pending) = PENDING.lock() {
        pending.retain(|(item, _)| item.id != display.id);
    }

    let verified = during.is_some_and(|info| info.current == raw);
    let back = after.is_some_and(|info| info.current == before.current);
    let ok = written.is_ok() && verified && restored.is_ok() && back;
    let value_json = |info: Option<vres_core::model::vibrance::VibranceInfo>| -> Value {
        info.map_or(Value::Null, |info| json!(info.current))
    };
    let report = json!({
        "ok": ok,
        "display": display.friendly_name,
        "backend": before.backend,
        "range": { "min": before.min, "max": before.max, "default": before.default },
        "original": before.current,
        "requested": { "value": value, "raw": raw },
        "write": { "ok": written.is_ok(), "error": written.as_ref().err().map(|e| e.to_string()), "ms": (write_ms * 100.0).round() / 100.0 },
        "readBack": value_json(during),
        "verified": verified,
        "restored": restored.is_ok(),
        "after": value_json(after),
        "backToOriginal": back,
    });
    let text = format!(
        "{} ({:?}): {} → {} (releído {:?}) → {} ({:.1} ms).",
        display.friendly_name,
        before.backend,
        before.current,
        raw,
        during.map(|info| info.current),
        after.map_or(-1, |info| info.current),
        write_ms
    );
    if ok {
        Report::ok(report, text)
    } else {
        Report::fail(report, text)
    }
}
