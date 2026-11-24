//! Comandos que prueban el hardware real: pantallas, matriz de color y rampas.
//!
//! Toda prueba que cambia algo lo restaura al terminar (guardián + Ctrl + C),
//! usa valores suaves y dura poco.

use std::time::{Duration, Instant};

use serde_json::{Value, json};
use vres_core::color::matrix::{self, ColorMatrix, DEFAULT_COEFFICIENT_LIMIT};
use vres_core::color::ramp::{GammaRamp, RampParams, RampRule, build_ramp};
use vres_core::model::display::DisplayInfo;
use vres_core::model::look::{DisplayTrim, EyeCare, Look, Tone};
use vres_win::{displays, gamma, mag::Magnifier, probe};

use crate::Report;
use crate::guard::RampGuard;

/// Tolerancia al releer una rampa: un escalón de 8 bits.
const RAMP_TOLERANCE: u16 = 0x0101;

fn ms(duration: Duration) -> f64 {
    (duration.as_secs_f64() * 1000.0 * 100.0).round() / 100.0
}

pub(crate) fn list_displays() -> Result<Vec<DisplayInfo>, Report> {
    displays::enumerate().map_err(|error| {
        Report::fail(
            json!({ "ok": false, "error": error.to_string() }),
            "No se pudieron enumerar las pantallas.",
        )
    })
}

/// Elige pantallas por número (1, 2…), nombre GDI (`\\.\DISPLAY1`) o
/// identificador estable (`d-…`). Sin selector, todas.
fn select<'a>(
    displays: &'a [DisplayInfo],
    selector: Option<&str>,
) -> Result<Vec<&'a DisplayInfo>, Report> {
    let Some(selector) = selector else {
        return Ok(displays.iter().collect());
    };
    let found = match selector.parse::<usize>() {
        Ok(number) => displays.get(number.wrapping_sub(1)),
        Err(_) => displays.iter().find(|display| {
            display.gdi_name.eq_ignore_ascii_case(selector) || display.id == selector
        }),
    };
    found.map(|display| vec![display]).ok_or_else(|| {
        Report::fail(
            json!({ "ok": false, "error": format!("display not found: {selector}") }),
            "No existe esa pantalla. Mira la lista con `vres-cli monitors`.",
        )
    })
}

fn ramp_summary(ramp: &GammaRamp) -> Value {
    json!({
        "identity": ramp.is_identity(),
        "maxDeviation": ramp.max_deviation(),
        "monotonic": ramp.is_monotonic(),
        "samples": {
            "r": [ramp.channels[0][0], ramp.channels[0][64], ramp.channels[0][128], ramp.channels[0][192], ramp.channels[0][255]],
            "g": [ramp.channels[1][0], ramp.channels[1][64], ramp.channels[1][128], ramp.channels[1][192], ramp.channels[1][255]],
            "b": [ramp.channels[2][0], ramp.channels[2][64], ramp.channels[2][128], ramp.channels[2][192], ramp.channels[2][255]],
        },
    })
}

fn saturation_matrix(saturation: f32, hue: f32) -> ColorMatrix {
    let mut look = Look {
        saturation,
        ..Look::default()
    };
    look.color.hue_shift_deg = hue;
    matrix::build_matrix(&look, None, DEFAULT_COEFFICIENT_LIMIT).matrix
}

/// `vres-cli monitors`
pub fn monitors() -> Report {
    match list_displays() {
        Ok(displays) => {
            let lines: Vec<String> = displays
                .iter()
                .enumerate()
                .map(|(index, display)| {
                    format!(
                        "{}. {} ({}) · {}×{} a {:.2} Hz · {:?} {} · {}{}{}",
                        index + 1,
                        display.friendly_name,
                        display.gdi_name,
                        display.bounds.width,
                        display.bounds.height,
                        display.refresh_hz,
                        display.adapter.vendor,
                        display.adapter.name,
                        if display.primary {
                            "principal"
                        } else {
                            "secundario"
                        },
                        if display.internal { " · interno" } else { "" },
                        if display.hdr.enabled { " · HDR" } else { "" },
                    )
                })
                .collect();
            Report::ok(
                json!({ "ok": true, "displays": displays }),
                lines.join("\n"),
            )
        }
        Err(report) => report,
    }
}

/// `vres-cli diag`: pantallas, estado de la matriz y de cada rampa.
pub fn diag() -> Report {
    let displays = match list_displays() {
        Ok(displays) => displays,
        Err(report) => return report,
    };

    let ramps: Vec<Value> = displays
        .iter()
        .map(|display| match gamma::read_ramp(&display.gdi_name) {
            Ok(ramp) => json!({ "display": display.gdi_name, "readable": true, "ramp": ramp_summary(&ramp) }),
            Err(error) => json!({ "display": display.gdi_name, "readable": false, "error": error.to_string() }),
        })
        .collect();

    let magnification = match Magnifier::init() {
        Ok(magnifier) => match magnifier.color_effect() {
            Ok(current) => json!({
                "available": true,
                "currentIsIdentity": current.is_identity(1e-4),
                "current": current.to_flat(),
            }),
            Err(error) => json!({ "available": true, "readError": error.to_string() }),
        },
        Err(error) => json!({ "available": false, "error": error.to_string() }),
    };

    let value = json!({
        "ok": true,
        "version": vres_core::APP_VERSION,
        "windows": vres_win::system::windows_version(),
        "displays": displays,
        "ramps": ramps,
        "magnification": magnification,
    });
    let text = serde_json::to_string_pretty(&value).unwrap_or_default();
    Report::ok(value, text)
}

/// `vres-cli mag get`
pub fn mag_get() -> Report {
    match Magnifier::init().and_then(|magnifier| magnifier.color_effect()) {
        Ok(current) => Report::ok(
            json!({ "ok": true, "identity": current.is_identity(1e-4), "matrix": current.0 }),
            format!("Matriz vigente: {:?}", current.0),
        ),
        Err(error) => Report::fail(
            json!({ "ok": false, "error": error.to_string() }),
            "No se pudo leer la matriz de color.",
        ),
    }
}

/// `vres-cli mag set`: aplica, verifica, mantiene y restaura.
pub fn mag_set(saturation: f32, hue: f32, hold: Duration) -> Report {
    let target = saturation_matrix(saturation, hue);
    let result = (|| -> vres_win::WinResult<Value> {
        let magnifier = Magnifier::init()?;
        let started = Instant::now();
        magnifier.set_color_effect(&target)?;
        let write_time = started.elapsed();
        let readback = magnifier.color_effect()?;
        probe::pump_for(hold);
        magnifier.set_color_effect(&ColorMatrix::IDENTITY)?;
        let restored = magnifier.color_effect()?;
        Ok(json!({
            "ok": readback.max_abs_diff(&target) < 1e-4 && restored.is_identity(1e-6),
            "writeMs": ms(write_time),
            "readbackMaxDiff": readback.max_abs_diff(&target),
            "restored": restored.is_identity(1e-6),
            "matrix": target.0,
        }))
    })();
    match result {
        Ok(value) if value["ok"] == true => {
            Report::ok(value, "La matriz se aplicó, se verificó y se restauró.")
        }
        Ok(value) => Report::fail(value, "La matriz no coincidió al releerla."),
        Err(error) => Report::fail(
            json!({ "ok": false, "error": error.to_string() }),
            "No se pudo aplicar la matriz de color.",
        ),
    }
}

/// `vres-cli mag reset`
pub fn mag_reset() -> Report {
    match Magnifier::init().and_then(|magnifier| magnifier.set_color_effect(&ColorMatrix::IDENTITY))
    {
        Ok(()) => Report::ok(json!({ "ok": true }), "Matriz de color restablecida."),
        Err(error) => Report::fail(
            json!({ "ok": false, "error": error.to_string() }),
            "No se pudo restablecer la matriz de color.",
        ),
    }
}

/// `vres-cli ramp get`
pub fn ramp_get(selector: Option<&str>, full: bool) -> Report {
    let displays = match list_displays() {
        Ok(displays) => displays,
        Err(report) => return report,
    };
    let targets = match select(&displays, selector) {
        Ok(targets) => targets,
        Err(report) => return report,
    };
    let ramps: Vec<Value> = targets
        .iter()
        .map(|display| match gamma::read_ramp(&display.gdi_name) {
            Ok(ramp) => {
                let mut value = json!({ "display": display.gdi_name, "name": display.friendly_name, "ramp": ramp_summary(&ramp) });
                if full {
                    value["channels"] = json!(ramp.channels.iter().map(|c| c.to_vec()).collect::<Vec<_>>());
                }
                value
            }
            Err(error) => json!({ "display": display.gdi_name, "error": error.to_string() }),
        })
        .collect();
    let value = json!({ "ok": true, "ramps": ramps });
    let text = serde_json::to_string_pretty(&value).unwrap_or_default();
    Report::ok(value, text)
}

/// Parámetros de `vres-cli ramp set`, en unidades de la interfaz.
#[derive(Debug, Clone, Copy)]
pub struct ToneArgs {
    pub gamma: f32,
    pub brightness: f32,
    pub contrast: f32,
    pub temperature: f32,
    pub shadows: f32,
    pub highlights: f32,
}

/// `vres-cli ramp set`: construye la curva, la verifica, la mantiene y
/// restaura la original.
pub fn ramp_set(selector: &str, args: ToneArgs, hold: Duration) -> Report {
    let displays = match list_displays() {
        Ok(displays) => displays,
        Err(report) => return report,
    };
    let display = match select(&displays, Some(selector)) {
        Ok(targets) => targets[0],
        Err(report) => return report,
    };
    let tone = Tone {
        gamma: args.gamma,
        brightness: args.brightness,
        contrast: args.contrast,
        temperature: args.temperature,
        shadows: args.shadows,
        highlights: args.highlights,
        ..Tone::default()
    };

    let result = (|| -> vres_win::WinResult<Value> {
        let guard = RampGuard::capture(&display.gdi_name)?;
        let params = RampParams::new(&tone, &EyeCare::default(), &DisplayTrim::default());
        let built = build_ramp(&params, guard.original(), RampRule::VRES);
        let write = gamma::write_ramp_verified(&display.gdi_name, &built.ramp, RAMP_TOLERANCE)?;
        std::thread::sleep(hold);
        drop(guard);
        let after = gamma::read_ramp(&display.gdi_name)?;
        Ok(json!({
            "ok": write.accepted && write.verified,
            "display": display.gdi_name,
            "accepted": write.accepted,
            "verified": write.verified,
            "readbackMaxDiff": write.max_diff,
            "writeMs": ms(write.write_time),
            "kept": built.kept,
            "requested": ramp_summary(&built.ramp),
            "restoredMaxDeviation": after.max_deviation(),
        }))
    })();
    match result {
        Ok(value) if value["ok"] == true => Report::ok(
            value,
            "La rampa se aplicó, se verificó y se restauró la original.",
        ),
        Ok(value) => Report::fail(value, "Windows no aplicó la rampa como se pidió."),
        Err(error) => Report::fail(
            json!({ "ok": false, "error": error.to_string() }),
            "No se pudo escribir la rampa.",
        ),
    }
}

/// `vres-cli ramp reset`: escribe la identidad (borra cualquier curva, incluida
/// una calibración).
pub fn ramp_reset(selector: Option<&str>) -> Report {
    let displays = match list_displays() {
        Ok(displays) => displays,
        Err(report) => return report,
    };
    let targets = match select(&displays, selector) {
        Ok(targets) => targets,
        Err(report) => return report,
    };
    let results: Vec<Value> = targets
        .iter()
        .map(|display| {
            match gamma::write_ramp_verified(
                &display.gdi_name,
                &GammaRamp::identity(),
                RAMP_TOLERANCE,
            ) {
                Ok(write) => json!({ "display": display.gdi_name, "ok": write.verified }),
                Err(error) => {
                    json!({ "display": display.gdi_name, "ok": false, "error": error.to_string() })
                }
            }
        })
        .collect();
    let ok = results.iter().all(|r| r["ok"] == true);
    let value = json!({ "ok": ok, "results": results });
    if ok {
        Report::ok(value, "Rampas restablecidas a la identidad.")
    } else {
        Report::fail(value, "Alguna rampa no se pudo restablecer.")
    }
}

/// `vres-cli probe ramp-limits`: qué tan lejos de la identidad acepta Windows
/// una rampa en cada pantalla.
pub fn probe_ramp_limits(selector: Option<&str>) -> Report {
    // Cada paso aceptado aclara la pantalla un instante: pocos pasos, separados
    // por más de medio segundo (muy por debajo de la zona de riesgo de 3 Hz).
    const DEVIATIONS: [u16; 7] = [64, 120, 127, 128, 129, 160, 255];
    const PAUSE: Duration = Duration::from_millis(600);
    let displays = match list_displays() {
        Ok(displays) => displays,
        Err(report) => return report,
    };
    let targets = match select(&displays, selector) {
        Ok(targets) => targets,
        Err(report) => return report,
    };

    let mut per_display = Vec::new();
    for display in targets {
        let result = (|| -> vres_win::WinResult<Value> {
            let guard = RampGuard::capture(&display.gdi_name)?;
            let mut steps = Vec::new();
            let mut max_accepted = 0u16;
            for deviation in DEVIATIONS {
                let mut ramp = GammaRamp::identity();
                for channel in &mut ramp.channels {
                    for (index, value) in channel.iter_mut().enumerate() {
                        *value = ((index as u16 + deviation).min(255)) * 257;
                    }
                }
                let write = gamma::write_ramp_verified(&display.gdi_name, &ramp, RAMP_TOLERANCE)?;
                let applied = write.accepted && write.verified;
                if applied {
                    max_accepted = max_accepted.max(deviation);
                }
                steps.push(json!({
                    "deviation": deviation,
                    "accepted": write.accepted,
                    "applied": write.verified,
                    "readbackMaxDiff": write.max_diff,
                }));
                gamma::write_ramp(&display.gdi_name, guard.original())?;
                std::thread::sleep(PAUSE);
            }
            Ok(json!({
                "display": display.gdi_name,
                "name": display.friendly_name,
                "vendor": display.adapter.vendor,
                "maxAppliedDeviation": max_accepted,
                "steps": steps,
            }))
        })();
        per_display.push(result.unwrap_or_else(
            |error| json!({ "display": display.gdi_name, "error": error.to_string() }),
        ));
    }
    let value = json!({ "ok": true, "displays": per_display });
    let text = serde_json::to_string_pretty(&value).unwrap_or_default();
    Report::ok(value, text)
}

fn percentiles(mut samples: Vec<Duration>) -> Value {
    if samples.is_empty() {
        return json!(null);
    }
    samples.sort();
    let at = |p: f64| ms(samples[((samples.len() - 1) as f64 * p).round() as usize]);
    json!({ "p50": at(0.5), "p95": at(0.95), "max": at(1.0), "samples": samples.len() })
}

/// `vres-cli probe timing`: cuánto tarda escribir una rampa y la matriz.
pub fn probe_timing(selector: Option<&str>, samples: usize) -> Report {
    let displays = match list_displays() {
        Ok(displays) => displays,
        Err(report) => return report,
    };
    let targets = match select(&displays, selector) {
        Ok(targets) => targets,
        Err(report) => return report,
    };

    let gentle_tone = Tone {
        gamma: 1.03,
        ..Tone::default()
    };
    let params = RampParams::new(&gentle_tone, &EyeCare::default(), &DisplayTrim::default());

    let mut ramps = Vec::new();
    for display in targets {
        let result = (|| -> vres_win::WinResult<Value> {
            let guard = RampGuard::capture(&display.gdi_name)?;
            let gentle = build_ramp(&params, guard.original(), RampRule::VRES).ramp;
            let mut times = Vec::with_capacity(samples);
            for index in 0..samples {
                let ramp = if index % 2 == 0 {
                    &gentle
                } else {
                    guard.original()
                };
                let started = Instant::now();
                gamma::write_ramp(&display.gdi_name, ramp)?;
                times.push(started.elapsed());
            }
            Ok(json!({ "display": display.gdi_name, "writeMs": percentiles(times) }))
        })();
        ramps.push(result.unwrap_or_else(
            |error| json!({ "display": display.gdi_name, "error": error.to_string() }),
        ));
    }

    let magnification = (|| -> vres_win::WinResult<Value> {
        let magnifier = Magnifier::init()?;
        let gentle = saturation_matrix(105.0, 0.0);
        let mut times = Vec::with_capacity(samples);
        for index in 0..samples {
            let target = if index % 2 == 0 {
                &gentle
            } else {
                &ColorMatrix::IDENTITY
            };
            let started = Instant::now();
            magnifier.set_color_effect(target)?;
            times.push(started.elapsed());
        }
        magnifier.set_color_effect(&ColorMatrix::IDENTITY)?;
        Ok(json!({ "writeMs": percentiles(times) }))
    })()
    .unwrap_or_else(|error| json!({ "error": error.to_string() }));

    let value = json!({ "ok": true, "ramps": ramps, "magnification": magnification });
    let text = serde_json::to_string_pretty(&value).unwrap_or_default();
    Report::ok(value, text)
}

/// Prueba de la matriz con la sonda de píxel.
fn selftest_matrix(primary: &DisplayInfo) -> Value {
    const PATCH: [u8; 3] = [200, 100, 50];
    let (x, y) = primary.bounds.center();
    let result = (|| -> vres_win::WinResult<Value> {
        let magnifier = Magnifier::init()?;
        let before = magnifier.color_effect()?;
        let patch = probe::ColorPatch::show(x, y, 48, PATCH)?;
        probe::pump_for(Duration::from_millis(300));
        let baseline = probe::read_screen_pixel(x, y)?;

        let target = saturation_matrix(130.0, 0.0);
        let started = Instant::now();
        magnifier.set_color_effect(&target)?;
        let write_time = started.elapsed();
        let readback = magnifier.color_effect()?;
        probe::pump_for(Duration::from_millis(300));
        let measured = probe::read_screen_pixel(x, y)?;

        magnifier.set_color_effect(&ColorMatrix::IDENTITY)?;
        probe::pump_for(Duration::from_millis(300));
        let restored = probe::read_screen_pixel(x, y)?;
        drop(patch);

        let expected = target
            .apply(baseline.map(|c| f32::from(c) / 255.0))
            .map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8);
        let max_error =
            |a: [u8; 3], b: [u8; 3]| (0..3).map(|i| a[i].abs_diff(b[i])).max().unwrap_or(0);
        let readback_ok = readback.max_abs_diff(&target) < 1e-4;
        let visible = max_error(measured, expected) <= 8;
        let restored_ok = max_error(restored, baseline) <= 2;

        Ok(json!({
            "ok": readback_ok && visible && restored_ok,
            "preexistingEffect": !before.is_identity(1e-4),
            "writeMs": ms(write_time),
            "readbackOk": readback_ok,
            "pixel": {
                "at": [x, y],
                "patch": PATCH,
                "baseline": baseline,
                "expected": expected,
                "measured": measured,
                "restored": restored,
                "visibleOnScreen": visible,
                "restoredOk": restored_ok,
            },
        }))
    })();
    result.unwrap_or_else(|error| json!({ "ok": false, "error": error.to_string() }))
}

/// Prueba de la rampa de una pantalla: escribir, releer, restaurar, releer.
fn selftest_ramp(display: &DisplayInfo) -> Value {
    let result = (|| -> vres_win::WinResult<Value> {
        let guard = RampGuard::capture(&display.gdi_name)?;
        let tone = Tone {
            gamma: 1.05,
            ..Tone::default()
        };
        let params = RampParams::new(&tone, &EyeCare::default(), &DisplayTrim::default());
        let built = build_ramp(&params, guard.original(), RampRule::VRES);
        let write = gamma::write_ramp_verified(&display.gdi_name, &built.ramp, RAMP_TOLERANCE)?;
        std::thread::sleep(Duration::from_millis(300));
        let restore =
            gamma::write_ramp_verified(&display.gdi_name, guard.original(), RAMP_TOLERANCE)?;
        drop(guard);
        Ok(json!({
            "ok": write.accepted && write.verified && restore.verified,
            "display": display.gdi_name,
            "name": display.friendly_name,
            "vendor": display.adapter.vendor,
            "hdr": display.hdr.enabled,
            "accepted": write.accepted,
            "verified": write.verified,
            "readbackMaxDiff": write.max_diff,
            "writeMs": ms(write.write_time),
            "restored": restore.verified,
        }))
    })();
    result.unwrap_or_else(
        |error| json!({ "ok": false, "display": display.gdi_name, "error": error.to_string() }),
    )
}

/// `vres-cli selftest`: aplica cada capa unos instantes con valores suaves,
/// la verifica (por lectura y, para la matriz, en pantalla) y la restaura.
pub fn selftest(layers: &[String]) -> Report {
    let displays = match list_displays() {
        Ok(displays) => displays,
        Err(report) => return report,
    };
    let wants = |layer: &str| layers.is_empty() || layers.iter().any(|l| l == layer);

    let mut value = json!({ "ok": true, "version": vres_core::APP_VERSION });
    let mut ok = true;

    if wants("mag") {
        let primary = displays.iter().find(|d| d.primary).or(displays.first());
        let result = match primary {
            Some(primary) => selftest_matrix(primary),
            None => json!({ "ok": false, "error": "no displays" }),
        };
        ok &= result["ok"] == true;
        value["mag"] = result;
    }
    if wants("ramp") {
        let results: Vec<Value> = displays.iter().map(selftest_ramp).collect();
        ok &= results.iter().all(|r| r["ok"] == true);
        value["ramp"] = json!(results);
    }
    value["ok"] = json!(ok);

    let text = serde_json::to_string_pretty(&value).unwrap_or_default();
    if ok {
        Report::ok(value, text)
    } else {
        Report::fail(value, text)
    }
}
