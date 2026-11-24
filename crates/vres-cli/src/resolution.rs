//! `vres-cli mode`, `hdr` y `probe mode-kill-revert`: resolución y HDR en
//! hardware real.
//!
//! Los cambios exigen `--allow-mode-switch` (la pantalla parpadea), usan solo
//! modos que el monitor lista, son temporales y se revierten solos al terminar.

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use vres_core::model::display::DisplayInfo;
use vres_core::model::resolution::{DisplayMode, ModeRequest, Scaling, is_listed};
use vres_win::{displays, modes};

use crate::Report;
use crate::guard::{HdrGuard, ModeGuard};

/// Tiempo máximo que una prueba mantiene un modo.
pub const MAX_MODE_HOLD_MS: u64 = 10_000;

fn ms(duration: Duration) -> f64 {
    (duration.as_secs_f64() * 1000.0 * 10.0).round() / 10.0
}

fn mode_json(mode: &DisplayMode) -> Value {
    json!(format!(
        "{}x{}@{}",
        mode.width, mode.height, mode.refresh_hz
    ))
}

fn failure(error: impl std::fmt::Display, text: &str) -> Report {
    Report::fail(json!({ "ok": false, "error": error.to_string() }), text)
}

fn find_display(selector: &str) -> Result<DisplayInfo, Report> {
    let found = displays::enumerate()
        .map_err(|error| failure(error, "No se pudieron enumerar las pantallas."))?;
    let display = match selector.parse::<usize>() {
        Ok(number) => found.get(number.wrapping_sub(1)),
        Err(_) => found.iter().find(|display| {
            display.gdi_name.eq_ignore_ascii_case(selector) || display.id == selector
        }),
    };
    display.cloned().ok_or_else(|| {
        failure(
            format!("display not found: {selector}"),
            "No existe esa pantalla. Mira la lista con `vres-cli monitors`.",
        )
    })
}

/// `1440x1080@200` o `1440x1080` (elige la frecuencia actual si existe; si no,
/// la más alta).
pub fn parse_mode(
    spec: &str,
    current: &DisplayMode,
    listed: &[DisplayMode],
) -> Option<DisplayMode> {
    let (size, rate) = match spec.split_once('@') {
        Some((size, rate)) => (size, Some(rate.trim().parse::<u32>().ok()?)),
        None => (spec, None),
    };
    let (width, height) = size.split_once(['x', 'X', '×'])?;
    let width = width.trim().parse().ok()?;
    let height = height.trim().parse().ok()?;
    let refresh_hz = match rate {
        Some(rate) => rate,
        None => {
            let rates: Vec<u32> = listed
                .iter()
                .filter(|mode| mode.width == width && mode.height == height)
                .map(|mode| mode.refresh_hz)
                .collect();
            if rates.contains(&current.refresh_hz) {
                current.refresh_hz
            } else {
                rates.into_iter().max()?
            }
        }
    };
    Some(DisplayMode::new(width, height, refresh_hz))
}

fn parse_scaling(name: &str) -> Option<Scaling> {
    match name {
        "stretch" | "stretched" => Some(Scaling::Stretched),
        "aspect" | "aspect-ratio" => Some(Scaling::AspectRatio),
        "center" | "centered" => Some(Scaling::Centered),
        "preferred" | "default" => Some(Scaling::Preferred),
        _ => None,
    }
}

fn request_for(display: &DisplayInfo, spec: &str, scaling: &str) -> Result<ModeRequest, Report> {
    let listed = modes::list_modes(&display.gdi_name);
    let current = modes::current_mode(&display.gdi_name)
        .map_err(|error| failure(error, "No se pudo leer el modo actual."))?;
    let mode = parse_mode(spec, &current, &listed).ok_or_else(|| {
        failure(
            format!("invalid or unlisted mode: {spec}"),
            "Modo inválido o que el monitor no lista. Formato: 1440x1080 o 1440x1080@200.",
        )
    })?;
    let scaling = parse_scaling(scaling).ok_or_else(|| {
        failure(
            format!("invalid scaling: {scaling}"),
            "Escalado inválido: stretch, aspect, center o preferred.",
        )
    })?;
    let request = ModeRequest {
        width: mode.width,
        height: mode.height,
        refresh_hz: mode.refresh_hz,
        scaling,
    };
    if !is_listed(&listed, &request) {
        return Err(failure(
            format!("unlisted mode: {spec}"),
            "El monitor no lista ese modo (las resoluciones personalizadas llegan en la v1.0).",
        ));
    }
    Ok(request)
}

/// `vres-cli mode list`.
pub fn mode_list(selector: Option<&str>) -> Report {
    let found = match displays::enumerate() {
        Ok(found) => found,
        Err(error) => return failure(error, "No se pudieron enumerar las pantallas."),
    };
    let chosen: Vec<&DisplayInfo> = match selector {
        Some(selector) => match find_display(selector) {
            Ok(display) => found.iter().filter(|d| d.id == display.id).collect(),
            Err(report) => return report,
        },
        None => found.iter().collect(),
    };
    let mut values = Vec::new();
    let mut text = String::new();
    for display in chosen {
        let gdi = &display.gdi_name;
        let list = modes::list_modes(gdi);
        let current = modes::current_mode(gdi).ok();
        let saved = modes::saved_mode(gdi).ok();
        let native = modes::native_mode(gdi);
        let scaling = modes::scaling(gdi).ok();
        let mut unique = list.clone();
        unique.dedup();
        values.push(json!({
            "id": display.id,
            "gdiName": gdi,
            "name": display.friendly_name,
            "current": current.as_ref().map(mode_json),
            "saved": saved.as_ref().map(mode_json),
            "native": native.as_ref().map(mode_json),
            "scaling": scaling,
            "count": list.len(),
            "duplicates": list.len() - unique.len(),
            "modes": list.iter().map(mode_json).collect::<Vec<_>>(),
        }));
        text.push_str(&format!(
            "{} ({gdi}) · actual {} · Windows {} · nativa {} · escalado {:?} · {} modos\n",
            display.friendly_name,
            current.map(|m| mode_json(&m)).unwrap_or(Value::Null),
            saved.map(|m| mode_json(&m)).unwrap_or(Value::Null),
            native.map(|m| mode_json(&m)).unwrap_or(Value::Null),
            scaling.unwrap_or_default(),
            list.len(),
        ));
    }
    let ok = values
        .iter()
        .all(|value| value["count"].as_u64().unwrap_or(0) > 0 && value["duplicates"] == 0);
    let value = json!({ "ok": ok, "displays": values });
    if ok {
        Report::ok(value, text.trim_end().to_owned())
    } else {
        Report::fail(value, text.trim_end().to_owned())
    }
}

/// `vres-cli mode test`: pregunta con `CDS_TEST`, sin cambiar nada.
pub fn mode_test(selector: &str, spec: &str, scaling: &str) -> Report {
    let display = match find_display(selector) {
        Ok(display) => display,
        Err(report) => return report,
    };
    let request = match request_for(&display, spec, scaling) {
        Ok(request) => request,
        Err(report) => return report,
    };
    let before = modes::current_mode(&display.gdi_name).ok();
    let result = modes::test_mode(&display.gdi_name, &request);
    let after = modes::current_mode(&display.gdi_name).ok();
    let unchanged = before == after;
    let value = json!({
        "ok": result.is_ok() && unchanged,
        "accepted": result.is_ok(),
        "error": result.as_ref().err().map(|e| e.to_string()),
        "unchanged": unchanged,
    });
    match result {
        Ok(()) => Report::ok(value, "Windows acepta ese modo (no se cambió nada)."),
        Err(_) => Report::fail(value, "Windows no acepta ese modo."),
    }
}

/// `vres-cli mode set`: aplica, relee el escalado, mantiene y revierte.
pub fn mode_set(selector: &str, spec: &str, scaling: &str, hold: Duration, allow: bool) -> Report {
    if !allow {
        return failure(
            "missing --allow-mode-switch",
            "Esta prueba cambia la resolución unos segundos: agrega --allow-mode-switch.",
        );
    }
    let display = match find_display(selector) {
        Ok(display) => display,
        Err(report) => return report,
    };
    let request = match request_for(&display, spec, scaling) {
        Ok(request) => request,
        Err(report) => return report,
    };
    let gdi = display.gdi_name.clone();
    let before_current = modes::current_mode(&gdi).ok();
    let before_saved = modes::saved_mode(&gdi).ok();
    let before_scaling = modes::scaling(&gdi).ok();

    let guard = ModeGuard::arm(&gdi);
    let started = Instant::now();
    let applied = modes::apply_scaled(&gdi, &request);
    let apply_ms = ms(started.elapsed());
    let during_mode = modes::current_mode(&gdi).ok();
    let during_scaling = modes::scaling(&gdi).ok();
    if applied.is_ok() {
        std::thread::sleep(hold);
    }

    let started = Instant::now();
    let reverted = guard.revert();
    let revert_ms = ms(started.elapsed());
    // Windows avisa el cambio en diferido: se deja asentar antes de releer.
    std::thread::sleep(Duration::from_millis(400));
    let after_current = modes::current_mode(&gdi).ok();
    let after_scaling = modes::scaling(&gdi).ok();
    let back = after_current.is_some() && after_current == before_saved;

    let scaling_ok = applied
        .as_ref()
        .is_ok_and(|got| !modes::scaling_ignored(request.scaling, *got));
    let ok = applied.is_ok() && reverted.is_ok() && back;
    let value = json!({
        "ok": ok,
        "display": display.friendly_name,
        "requested": { "mode": mode_json(&request.mode()), "scaling": request.scaling },
        "before": {
            "current": before_current.as_ref().map(mode_json),
            "saved": before_saved.as_ref().map(mode_json),
            "scaling": before_scaling,
        },
        "applied": {
            "ok": applied.is_ok(),
            "error": applied.as_ref().err().map(|e| e.to_string()),
            "ms": apply_ms,
            "mode": during_mode.as_ref().map(mode_json),
            "scaling": during_scaling,
            "scalingAsRequested": scaling_ok,
        },
        "reverted": {
            "ok": reverted.is_ok(),
            "error": reverted.as_ref().err().map(|e| e.to_string()),
            "ms": revert_ms,
            "mode": after_current.as_ref().map(mode_json),
            "scaling": after_scaling,
            "backToSaved": back,
            "scalingRestored": after_scaling == before_scaling,
        },
    });
    let text = format!(
        "{}: {} {:?} → {} ({} ms, escalado {:?}) → volvió a {} ({} ms, escalado {:?}).",
        display.friendly_name,
        mode_json(&request.mode()),
        request.scaling,
        if applied.is_ok() {
            "aplicado"
        } else {
            "falló"
        },
        apply_ms,
        during_scaling.unwrap_or_default(),
        after_current.map(|m| mode_json(&m)).unwrap_or(Value::Null),
        revert_ms,
        after_scaling.unwrap_or_default(),
    );
    if ok {
        Report::ok(value, text)
    } else {
        Report::fail(value, text)
    }
}

/// Subcomando oculto de `probe mode-kill-revert`: aplica un modo sin
/// guardián, avisa por la salida estándar y espera a que lo maten.
pub fn mode_hold(selector: &str, spec: &str, scaling: &str) -> Report {
    let display = match find_display(selector) {
        Ok(display) => display,
        Err(report) => return report,
    };
    let request = match request_for(&display, spec, scaling) {
        Ok(request) => request,
        Err(report) => return report,
    };
    if let Err(error) = modes::apply_scaled(&display.gdi_name, &request) {
        return failure(error, "No se pudo aplicar el modo.");
    }
    println!("applied");
    // Tope de seguridad: si nadie lo mata, vuelve solo.
    std::thread::sleep(Duration::from_secs(20));
    let _ = modes::revert_to_saved(&display.gdi_name);
    Report::ok(json!({ "ok": true }), "")
}

/// `vres-cli probe mode-kill-revert`: ¿Windows revierte solo el modo temporal
/// si el proceso que lo aplicó muere?
pub fn probe_mode_kill_revert(selector: &str, spec: &str, scaling: &str, allow: bool) -> Report {
    if !allow {
        return failure(
            "missing --allow-mode-switch",
            "Esta prueba cambia la resolución unos segundos: agrega --allow-mode-switch.",
        );
    }
    let display = match find_display(selector) {
        Ok(display) => display,
        Err(report) => return report,
    };
    if let Err(report) = request_for(&display, spec, scaling) {
        return report;
    }
    let gdi = display.gdi_name.clone();
    let saved = modes::saved_mode(&gdi).ok();
    let Ok(exe) = std::env::current_exe() else {
        return failure("current_exe", "No se encontró el ejecutable.");
    };

    // Por si algo sale mal, este proceso vuelve la pantalla al terminar.
    let guard = ModeGuard::arm(&gdi);
    let child = Command::new(exe)
        .args([
            "internal-mode-hold",
            "--display",
            &gdi,
            "--mode",
            spec,
            "--scaling",
            scaling,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(error) => return failure(error, "No se pudo lanzar el proceso de prueba."),
    };
    let applied = child.stdout.take().is_some_and(|stdout| {
        BufReader::new(stdout)
            .lines()
            .next()
            .is_some_and(|line| line.is_ok_and(|line| line.trim() == "applied"))
    });
    let during = modes::current_mode(&gdi).ok();
    std::thread::sleep(Duration::from_millis(800));
    let _ = child.kill();
    let _ = child.wait();
    // Se le da tiempo a Windows para reaccionar a la muerte del proceso.
    std::thread::sleep(Duration::from_millis(2500));
    let after_kill = modes::current_mode(&gdi).ok();
    let reverted_by_windows = after_kill.is_some() && after_kill == saved;
    let restored = guard.revert();
    std::thread::sleep(Duration::from_millis(400));
    let final_mode = modes::current_mode(&gdi).ok();

    let ok = applied && restored.is_ok() && final_mode == saved;
    let value = json!({
        "ok": ok,
        "applied": applied,
        "during": during.as_ref().map(mode_json),
        "afterKill": after_kill.as_ref().map(mode_json),
        "revertedByWindows": reverted_by_windows,
        "final": final_mode.as_ref().map(mode_json),
    });
    let text = if reverted_by_windows {
        "Windows revirtió solo el modo al morir el proceso."
    } else {
        "Windows NO revierte el modo al morir el proceso: VRes lo restaura al volver a abrir (restore-modes.json)."
    };
    if ok {
        Report::ok(value, text)
    } else {
        Report::fail(value, format!("{text} (la prueba no terminó bien)"))
    }
}

/// Espera un evento del motor que cumpla `wanted` (con tope).
fn wait_event(
    events: &std::sync::mpsc::Receiver<vres_engine::EngineEvent>,
    timeout: Duration,
    wanted: impl Fn(&vres_engine::EngineEvent) -> bool,
) -> Option<vres_engine::EngineEvent> {
    let deadline = Instant::now() + timeout;
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        match events.recv_timeout(left) {
            Ok(event) if wanted(&event) => return Some(event),
            Ok(_) => {}
            Err(_) => return None,
        }
    }
    None
}

/// `vres-cli probe engine-modes`: el motor real, en este proceso, aplica un
/// modo desde su hilo auxiliar, lo deja vencer, lo vuelve a aplicar, lo
/// confirma, vuelve a la de Windows y se apaga. Sin tocar los archivos de
/// estado de la app instalada.
pub fn probe_engine_modes(selector: &str, spec: &str, allow: bool) -> Report {
    use std::sync::{Arc, mpsc};
    use vres_core::model::resolution::ModeOutcome;
    use vres_core::planner::Requested;
    use vres_engine::{Command as EngineCommand, EngineConfig, EngineEvent, EngineSettings};

    if !allow {
        return failure(
            "missing --allow-mode-switch",
            "Esta prueba cambia la resolución varias veces: agrega --allow-mode-switch.",
        );
    }
    let display = match find_display(selector) {
        Ok(display) => display,
        Err(report) => return report,
    };
    let request = match request_for(&display, spec, "stretch") {
        Ok(request) => request,
        Err(report) => return report,
    };
    let gdi = display.gdi_name.clone();
    let saved = modes::saved_mode(&gdi).ok();
    // Por si algo falla a mitad de camino.
    let guard = ModeGuard::arm(&gdi);

    let (events_tx, events_rx) = mpsc::channel();
    let sink: vres_engine::EventSink = Arc::new(move |event| {
        let _ = events_tx.send(event);
    });
    let config = EngineConfig {
        restore_path: None,
        modes_restore_path: None,
        requested: Requested {
            color_enabled: false,
            ..Requested::default()
        },
        settings: EngineSettings {
            confirm_seconds: 4,
            tones: false,
            ..EngineSettings::default()
        },
    };
    let (mut engine, _snapshot) = match vres_engine::spawn(config, sink) {
        Ok(started) => started,
        Err(error) => return failure(error, "El motor no pudo arrancar."),
    };
    fn ask<T>(
        engine: &vres_engine::EngineHandle,
        make: impl FnOnce(vres_engine::Reply<T>) -> EngineCommand,
    ) -> Option<Result<T, vres_core::ipc::ErrorCode>> {
        let (tx, rx) = mpsc::channel();
        engine.send(make(tx));
        rx.recv_timeout(Duration::from_secs(15)).ok()
    }

    // 1. Probar: queda pendiente y el modo sigue aplicado aunque el hilo
    //    auxiliar ya terminó.
    let first = ask(&engine, |reply| EngineCommand::ModeApply {
        display_id: display.id.clone(),
        request,
        reply,
    });
    let first_ok = matches!(first, Some(Ok(_)));
    std::thread::sleep(Duration::from_millis(1500));
    let held = modes::current_mode(&gdi).ok();
    let held_ok = held == Some(request.mode());

    // 2. Sin respuesta: vence y vuelve solo.
    let expired = wait_event(&events_rx, Duration::from_secs(8), |event| {
        matches!(event, EngineEvent::ModeResolved(resolved) if resolved.outcome == ModeOutcome::Expired)
    })
    .is_some();
    std::thread::sleep(Duration::from_millis(500));
    let after_expiry = modes::current_mode(&gdi).ok();

    // 3. Probar y conservar; después, volver a la de Windows.
    std::thread::sleep(Duration::from_secs(2));
    let second = ask(&engine, |reply| EngineCommand::ModeApply {
        display_id: display.id.clone(),
        request,
        reply,
    });
    let token = match &second {
        Some(Ok(pending)) => pending.token.clone(),
        _ => String::new(),
    };
    let kept = matches!(
        ask(&engine, |reply| EngineCommand::ModeConfirm {
            token: token.clone(),
            reply
        }),
        Some(Ok(()))
    );
    std::thread::sleep(Duration::from_millis(1500));
    let kept_mode = modes::current_mode(&gdi).ok();
    let restored = matches!(
        ask(&engine, |reply| EngineCommand::ModeRestore {
            display_id: display.id.clone(),
            reply
        }),
        Some(Ok(()))
    );
    std::thread::sleep(Duration::from_millis(500));
    let after_restore = modes::current_mode(&gdi).ok();

    let stopped = engine.shutdown(Duration::from_secs(6));
    drop(guard);
    std::thread::sleep(Duration::from_millis(400));
    let final_mode = modes::current_mode(&gdi).ok();

    let ok = first_ok
        && held_ok
        && expired
        && after_expiry == saved
        && kept
        && kept_mode == Some(request.mode())
        && restored
        && after_restore == saved
        && stopped
        && final_mode == saved;
    let value = json!({
        "ok": ok,
        "applied": first_ok,
        "heldAfterWorkerExit": held_ok,
        "expiredByItself": expired,
        "afterExpiry": after_expiry.as_ref().map(mode_json),
        "kept": kept,
        "keptMode": kept_mode.as_ref().map(mode_json),
        "restored": restored,
        "afterRestore": after_restore.as_ref().map(mode_json),
        "engineStopped": stopped,
        "final": final_mode.as_ref().map(mode_json),
    });
    let text = if ok {
        "El motor aplica, mantiene, vence, conserva y restaura el modo correctamente."
    } else {
        "El ciclo de resolución del motor no terminó bien: mira el JSON."
    };
    if ok {
        Report::ok(value, text)
    } else {
        Report::fail(value, text)
    }
}

/// `vres-cli hdr get`.
pub fn hdr_get() -> Report {
    match displays::enumerate() {
        Ok(found) => {
            let list: Vec<Value> = found
                .iter()
                .map(|display| {
                    json!({
                        "id": display.id,
                        "name": display.friendly_name,
                        "supported": display.hdr.supported,
                        "enabled": display.hdr.enabled,
                    })
                })
                .collect();
            let text = found
                .iter()
                .map(|display| {
                    format!(
                        "{}: HDR {}{}",
                        display.friendly_name,
                        if display.hdr.enabled {
                            "activo"
                        } else {
                            "apagado"
                        },
                        if display.hdr.supported {
                            ""
                        } else {
                            " (no admite)"
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            Report::ok(json!({ "ok": true, "displays": list }), text)
        }
        Err(error) => failure(error, "No se pudieron enumerar las pantallas."),
    }
}

/// `vres-cli hdr set`: cambia el HDR, lo relee, mantiene y lo deja como estaba.
pub fn hdr_set(selector: &str, enabled: bool, hold: Duration, allow: bool) -> Report {
    if !allow {
        return failure(
            "missing --allow-mode-switch",
            "Esta prueba cambia el HDR unos segundos: agrega --allow-mode-switch.",
        );
    }
    let display = match find_display(selector) {
        Ok(display) => display,
        Err(report) => return report,
    };
    if !display.hdr.supported {
        return failure("hdr unsupported", "Esa pantalla no admite HDR.");
    }
    let original = display.hdr.enabled;
    let guard = HdrGuard::arm(&display.gdi_name, original);
    let started = Instant::now();
    let result = modes::set_hdr(&display.gdi_name, enabled);
    let set_ms = ms(started.elapsed());
    std::thread::sleep(Duration::from_millis(1500));
    let during = find_display(&display.id).ok().map(|d| d.hdr.enabled);
    std::thread::sleep(hold);
    let restored = guard.restore();
    std::thread::sleep(Duration::from_millis(1500));
    let after = find_display(&display.id).ok().map(|d| d.hdr.enabled);

    let ok =
        result.is_ok() && during == Some(enabled) && restored.is_ok() && after == Some(original);
    let value = json!({
        "ok": ok,
        "original": original,
        "requested": enabled,
        "set": { "ok": result.is_ok(), "error": result.as_ref().err().map(|e| e.to_string()), "ms": set_ms },
        "during": during,
        "restored": restored.is_ok(),
        "after": after,
    });
    let text = format!(
        "{}: HDR {original} → {:?} → {:?}",
        display.friendly_name, during, after
    );
    if ok {
        Report::ok(value, text)
    } else {
        Report::fail(value, text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modes_with_and_without_rate() {
        let current = DisplayMode::new(1920, 1080, 200);
        let listed = [
            DisplayMode::new(1440, 1080, 200),
            DisplayMode::new(1440, 1080, 60),
            DisplayMode::new(1280, 960, 60),
        ];
        assert_eq!(
            parse_mode("1440x1080", &current, &listed),
            Some(DisplayMode::new(1440, 1080, 200))
        );
        assert_eq!(
            parse_mode("1280X960", &current, &listed),
            Some(DisplayMode::new(1280, 960, 60)),
            "sin la frecuencia actual, la más alta"
        );
        assert_eq!(
            parse_mode("1440x1080@60", &current, &listed),
            Some(DisplayMode::new(1440, 1080, 60))
        );
        assert_eq!(parse_mode("1440", &current, &listed), None);
        assert_eq!(parse_mode("1600x900", &current, &listed), None);
    }

    #[test]
    fn scaling_names() {
        assert_eq!(parse_scaling("stretch"), Some(Scaling::Stretched));
        assert_eq!(parse_scaling("aspect"), Some(Scaling::AspectRatio));
        assert_eq!(parse_scaling("zoom"), None);
    }
}
