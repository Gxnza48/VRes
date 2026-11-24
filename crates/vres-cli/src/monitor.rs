//! `vres-cli ddc get` y `vres-cli ddc test`: controles por hardware de cada
//! pantalla (DDC/CI en monitores externos, WMI en el panel de una notebook).

use std::time::Instant;

use serde_json::{Value, json};
use vres_core::model::display::DisplayInfo;
use vres_core::monitor::DdcControl;
use vres_win::{ddc, panel};

use crate::Report;
use crate::hw::list_displays;

fn ms(started: Instant) -> f64 {
    (started.elapsed().as_secs_f64() * 10_000.0).round() / 10.0
}

fn details(display: &DisplayInfo) -> Value {
    let started = Instant::now();
    let (controls, error) = if display.internal {
        (Default::default(), None)
    } else {
        match ddc::read_all(&display.gdi_name) {
            Ok(controls) => (controls, None),
            Err(error) => (Default::default(), Some(error.to_string())),
        }
    };
    let ddc_ms = ms(started);
    let started = Instant::now();
    let panel = if display.internal {
        panel::brightness().map_err(|error| error.to_string())
    } else {
        Ok(None)
    };
    json!({
        "name": display.friendly_name,
        "gdiName": display.gdi_name,
        "internal": display.internal,
        "scalePercent": ddc::scale_percent(&display.gdi_name),
        "ddc": controls,
        "ddcMs": ddc_ms,
        "ddcError": error,
        "panelBrightness": panel.as_ref().ok().copied().flatten(),
        "panelError": panel.err(),
        "panelMs": ms(started),
    })
}

/// Lo que se puede ajustar por hardware en cada pantalla (solo lee).
pub fn get(display: Option<&str>) -> Report {
    let displays = match list_displays() {
        Ok(displays) => displays,
        Err(report) => return report,
    };
    let list: Vec<Value> = displays
        .iter()
        .filter(|candidate| {
            display.is_none_or(|wanted| candidate.id == wanted || candidate.gdi_name == wanted)
        })
        .map(details)
        .collect();
    let value = json!({ "ok": true, "displays": list });
    let text = serde_json::to_string_pretty(&value).unwrap_or_default();
    Report::ok(value, text)
}

/// Brillo ±1 y vuelta, en cada pantalla que lo permite (DDC/CI o WMI).
pub fn test() -> Report {
    let displays = match list_displays() {
        Ok(displays) => displays,
        Err(report) => return report,
    };
    let mut results = Vec::new();
    for display in &displays {
        if display.internal {
            let result = (|| -> vres_win::WinResult<Value> {
                let Some(before) = panel::brightness()? else {
                    return Ok(json!({ "skipped": "no panel brightness" }));
                };
                let probe = if before >= 100 {
                    before - 1
                } else {
                    before + 1
                };
                let started = Instant::now();
                panel::set_brightness(probe)?;
                let write_ms = ms(started);
                std::thread::sleep(std::time::Duration::from_millis(300));
                let during = panel::brightness()?;
                panel::set_brightness(before)?;
                std::thread::sleep(std::time::Duration::from_millis(300));
                let after = panel::brightness()?;
                Ok(json!({
                    "ok": during == Some(probe) && after == Some(before),
                    "before": before, "probe": probe, "during": during, "after": after,
                    "writeMs": write_ms,
                }))
            })();
            results.push(json!({
                "name": display.friendly_name,
                "kind": "panel",
                "result": result.unwrap_or_else(|error| json!({ "ok": false, "error": error.to_string() })),
            }));
            continue;
        }
        let result = (|| -> vres_win::WinResult<Value> {
            let Some(before) = ddc::get(&display.gdi_name, DdcControl::Brightness)? else {
                return Ok(json!({ "skipped": "no DDC/CI" }));
            };
            let probe = if before.current >= before.max {
                before.current - 1
            } else {
                before.current + 1
            };
            let started = Instant::now();
            let during = ddc::set(&display.gdi_name, DdcControl::Brightness, probe)?;
            let write_ms = ms(started);
            let after = ddc::set(&display.gdi_name, DdcControl::Brightness, before.current)?;
            Ok(json!({
                "ok": during.current == probe && after.current == before.current,
                "before": before.current, "max": before.max, "probe": probe,
                "during": during.current, "after": after.current,
                "writeMs": write_ms,
            }))
        })();
        results.push(json!({
            "name": display.friendly_name,
            "kind": "ddc",
            "result": result.unwrap_or_else(|error| json!({ "ok": false, "error": error.to_string() })),
        }));
    }
    let ok = results.iter().all(|entry| entry["result"]["ok"] != false);
    let value = json!({ "ok": ok, "displays": results });
    let text = serde_json::to_string_pretty(&value).unwrap_or_default();
    if ok {
        Report::ok(value, text)
    } else {
        Report::fail(value, text)
    }
}
