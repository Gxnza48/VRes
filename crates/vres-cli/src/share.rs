//! `vres-cli share`: lee y arma códigos para compartir (no toca la pantalla).

use serde_json::json;
use vres_core::crosshair::Crosshair;
use vres_core::model::look::Look;
use vres_core::share::{self, ShareKind};

use crate::Report;

/// `vres-cli share decode CODE`: muestra qué hay en un código.
pub fn decode(code: &str) -> Report {
    let kind = match share::kind_of(code) {
        Ok(kind) => kind,
        Err(error) => {
            return Report::fail(
                json!({ "ok": false, "error": error }),
                format!("No es un código de VRes ({error})."),
            );
        }
    };
    let result = match kind {
        ShareKind::Look => {
            share::decode_look(code).map(|d| (json!({ "look": d.value }), d.clamped))
        }
        ShareKind::Crosshair => {
            share::decode_crosshair(code).map(|d| (json!({ "crosshair": d.value }), d.clamped))
        }
        ShareKind::Resolution => {
            share::decode_resolution(code).map(|d| (json!({ "resolution": d.value }), d.clamped))
        }
    };
    match result {
        Ok((mut value, clamped)) => {
            value["ok"] = json!(true);
            value["kind"] = json!(kind);
            value["clamped"] = json!(clamped);
            let text = serde_json::to_string_pretty(&value).unwrap_or_default();
            Report::ok(value, text)
        }
        Err(error) => Report::fail(
            json!({ "ok": false, "kind": kind, "error": error }),
            format!("El código está dañado ({error})."),
        ),
    }
}

/// `vres-cli share encode --look JSON` o `--crosshair JSON`.
pub fn encode(look: Option<&str>, crosshair: Option<&str>) -> Report {
    let code = match (look, crosshair) {
        (Some(json), None) => serde_json::from_str::<Look>(json)
            .map(|look| Ok(share::encode_look(&look)))
            .map_err(|error| error.to_string()),
        (None, Some(json)) => serde_json::from_str::<Crosshair>(json)
            .map(|crosshair| share::encode_crosshair(&crosshair).map_err(|e| e.to_string()))
            .map_err(|error| error.to_string()),
        _ => Err("use --look or --crosshair".to_owned()),
    };
    match code {
        Ok(Ok(code)) => Report::ok(json!({ "ok": true, "code": code }), code),
        Ok(Err(error)) | Err(error) => Report::fail(
            json!({ "ok": false, "error": error }),
            "No se pudo armar el código: pasa un JSON con --look o --crosshair.",
        ),
    }
}

/// `vres-cli share presets`: los crosshairs de fábrica con su código.
pub fn presets() -> Report {
    let list: Vec<_> = vres_core::crosshair::presets()
        .into_iter()
        .filter_map(|preset| {
            let code = share::encode_crosshair(&preset.crosshair).ok()?;
            Some(json!({ "id": preset.id, "code": code, "crosshair": preset.crosshair }))
        })
        .collect();
    let text = list
        .iter()
        .map(|item| {
            format!(
                "{}  {}",
                item["id"].as_str().unwrap_or_default(),
                item["code"].as_str().unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Report::ok(json!({ "ok": true, "presets": list }), text)
}
