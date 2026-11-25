//! `vres-cli lut`: exporta un look como LUT 3D (`.cube`) para editores de
//! video. No toca la pantalla.

use std::path::{Path, PathBuf};

use serde_json::json;
use vres_core::color::lut::{LUT_SIZES, LutVibrance, look_lut};
use vres_core::model::look::Look;
use vres_core::model::state::SessionState;
use vres_core::share;

use crate::Report;

/// De dónde sale el look.
pub enum Source<'a> {
    /// Un código para compartir (`VRL1-…`).
    Code(&'a str),
    /// El look en JSON.
    Json(&'a str),
    /// El que VRes tiene pedido ahora (su sesión guardada).
    Current,
}

/// `%LOCALAPPDATA%\io.github.gxnza48.vres\state\session.json`.
fn session_path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|dir| {
        PathBuf::from(dir)
            .join(vres_core::APP_IDENTIFIER)
            .join("state")
            .join("session.json")
    })
}

fn load(source: &Source<'_>) -> Result<Look, String> {
    match source {
        Source::Code(code) => share::decode_look(code)
            .map(|decoded| decoded.value)
            .map_err(|error| format!("bad look code: {error}")),
        Source::Json(text) => {
            serde_json::from_str::<Look>(text).map_err(|error| format!("bad look JSON: {error}"))
        }
        Source::Current => {
            // Solo lectura: sin la cuarentena de `store::read_json`.
            let path = session_path().ok_or("LOCALAPPDATA is not set")?;
            let text = std::fs::read_to_string(&path)
                .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
            serde_json::from_str::<SessionState>(&text)
                .map(|session| session.requested.look)
                .map_err(|error| format!("bad session.json: {error}"))
        }
    }
}

/// `vres-cli lut --code VRL1-… --out look.cube`.
pub fn export(source: &Source<'_>, out: &Path, size: usize, vibrance: &str) -> Report {
    let vibrance = match vibrance {
        "emulated" => LutVibrance::Emulated,
        "off" => LutVibrance::Off,
        _ => {
            return Report::fail(
                json!({ "ok": false, "error": "vibrance must be emulated or off" }),
                "--vibrance tiene que ser emulated u off.",
            );
        }
    };
    let look = match load(source) {
        Ok(look) => look,
        Err(error) => {
            return Report::fail(
                json!({ "ok": false, "error": error }),
                format!("No se pudo leer el look ({error})."),
            );
        }
    };
    let size = size.clamp(*LUT_SIZES.start(), *LUT_SIZES.end());
    let lut = look_lut(&look, size, vibrance);
    let title = out
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "VRes".to_owned());
    if let Err(error) = std::fs::write(out, lut.to_cube(&format!("VRes {title}"))) {
        return Report::fail(
            json!({ "ok": false, "error": error.to_string() }),
            format!("No se pudo escribir {}: {error}", out.display()),
        );
    }
    Report::ok(
        json!({
            "ok": true,
            "out": out.display().to_string(),
            "size": size,
            "vibrance": if vibrance == LutVibrance::Emulated { "emulated" } else { "off" },
            "look": look,
        }),
        format!(
            "LUT de {size}³ guardado en {} (saturación {:.0} %, vibrance {:.0}).",
            out.display(),
            look.saturation,
            look.vibrance
        ),
    )
}
