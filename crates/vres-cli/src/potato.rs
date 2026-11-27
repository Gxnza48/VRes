//! `vres-cli potato`: gráficos potato de NVIDIA (perfil del juego en el driver).

use serde_json::json;
use vres_core::potato::{self, PotatoLevel};
use vres_win::nvdrs;

use crate::Report;

/// `vres-cli potato status`: qué nivel tiene cada juego (solo lee).
pub fn status() -> Report {
    let games: Vec<_> = potato::games().iter().map(nvdrs::potato_status).collect();
    let text = games
        .iter()
        .map(|status| {
            if status.available {
                format!(
                    "{}: {:?}{}",
                    status.game.name,
                    status.level,
                    if status.custom {
                        " (con ajustes propios)"
                    } else {
                        ""
                    }
                )
            } else {
                format!("{}: no hay driver de NVIDIA con perfiles", status.game.name)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let driver = nvdrs::driver_version();
    let text = match driver {
        Some(version) => format!(
            "{text}\nDriver de NVIDIA: {}.{:02}{}",
            version / 100,
            version % 100,
            if potato::legacy_driver(version) {
                " (viejo: también se cambia el archivo del juego)"
            } else {
                ""
            }
        ),
        None => text,
    };
    Report::ok(
        json!({ "ok": true, "games": games, "driverVersion": driver }),
        text,
    )
}

/// `vres-cli potato set LEVEL`: pone el nivel en ROB RUN.
pub fn set(level: &str) -> Report {
    let level = match level {
        "off" => PotatoLevel::Off,
        "low" => PotatoLevel::Low,
        "potato" => PotatoLevel::Potato,
        other => {
            return Report::fail(
                json!({ "ok": false, "error": "bad level" }),
                format!("Nivel desconocido: {other} (usa off, low o potato)."),
            );
        }
    };
    let Some(game) = potato::games().into_iter().next() else {
        return Report::fail(json!({ "ok": false }), "No hay juegos con potato.");
    };
    match nvdrs::potato_apply(&game, level) {
        Ok(()) => {
            let status = nvdrs::potato_status(&game);
            let ok = status.level == level;
            let value = json!({ "ok": ok, "status": status });
            let text = format!("{}: {:?} (releído: {:?})", game.name, level, status.level);
            if ok {
                Report::ok(value, text)
            } else {
                Report::fail(value, text)
            }
        }
        Err(error) => Report::fail(
            json!({ "ok": false, "error": error.to_string(), "needsAdmin": nvdrs::needs_admin(&error) }),
            format!("No se pudo guardar en el driver: {error}"),
        ),
    }
}
