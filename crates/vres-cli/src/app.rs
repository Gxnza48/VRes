//! Comandos que le hablan a la instancia de VRes en ejecución.

use std::time::{Duration, Instant};

use serde_json::json;
use vres_win::control::{self, ACK, RemoteCommand, ShutdownOutcome};

use crate::Report;

const SEND_TIMEOUT: Duration = Duration::from_secs(2);

/// `vres-cli status`
pub fn status() -> Report {
    match control::send(RemoteCommand::Ping, SEND_TIMEOUT) {
        Ok(Some(answer)) => Report::ok(
            json!({ "ok": true, "running": true, "responding": answer == ACK }),
            "VRes está corriendo y responde.",
        ),
        Ok(None) => Report::ok(
            json!({ "ok": true, "running": false }),
            "VRes no está corriendo.",
        ),
        Err(error) => Report::fail(
            json!({ "ok": false, "running": true, "responding": false, "error": error.to_string() }),
            "VRes está corriendo pero no responde.",
        ),
    }
}

/// `vres-cli reset`
pub fn reset() -> Report {
    match control::send(RemoteCommand::EmergencyReset, SEND_TIMEOUT) {
        Ok(Some(_)) => Report::ok(
            json!({ "ok": true, "running": true }),
            "Listo: VRes restableció la pantalla.",
        ),
        // Sin instancia en ejecución: se deshace lo que haya quedado de un
        // cierre inesperado (restore.json).
        Ok(None) => {
            let recovered = vres_engine::recover_from_disk();
            let message = if recovered {
                "VRes no estaba corriendo: se restauraron los colores que quedaron de un cierre inesperado."
            } else {
                "VRes no está corriendo y no había cambios pendientes."
            };
            Report::ok(
                json!({ "ok": true, "running": false, "recovered": recovered }),
                message,
            )
        }
        Err(error) => Report::fail(
            json!({ "ok": false, "error": error.to_string() }),
            "No se pudo contactar a VRes.",
        ),
    }
}

/// `vres-cli shutdown`
pub fn shutdown(timeout: Duration) -> Report {
    let started = Instant::now();
    match control::shutdown_running(timeout) {
        Ok(outcome) => {
            let (ok, message) = match outcome {
                ShutdownOutcome::NotRunning => (true, "VRes no está corriendo."),
                ShutdownOutcome::Exited => (true, "VRes se cerró."),
                ShutdownOutcome::StillRunning => {
                    (false, "VRes no terminó dentro del tiempo de espera.")
                }
            };
            let value = json!({
                "ok": ok,
                "outcome": format!("{outcome:?}"),
                "elapsedMs": started.elapsed().as_millis(),
            });
            if ok {
                Report::ok(value, message)
            } else {
                Report::fail(value, message)
            }
        }
        Err(error) => Report::fail(
            json!({ "ok": false, "error": error.to_string() }),
            "No se pudo contactar a VRes.",
        ),
    }
}
