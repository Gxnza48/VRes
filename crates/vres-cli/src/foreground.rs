//! `vres-cli fg watch`: qué programa está en primer plano y si es un juego
//! conocido. No abre ningún proceso.

use std::time::{Duration, Instant};

use serde_json::json;
use vres_core::profiles::match_game;
use vres_win::process;

use crate::Report;

/// Mira el primer plano durante `duration` y anota cada cambio.
pub fn watch(duration: Duration) -> Report {
    let start = Instant::now();
    let mut seen: Vec<serde_json::Value> = Vec::new();
    let mut text = String::new();
    let mut last: Option<String> = None;
    while start.elapsed() < duration {
        let current = process::foreground_process();
        let exe = current.as_ref().map(|process| process.exe.clone());
        if exe != last {
            let game = exe.as_deref().and_then(|exe| match_game(exe, &[]));
            let at = start.elapsed().as_millis() as u64;
            seen.push(json!({
                "ms": at,
                "exe": exe,
                "pid": current.as_ref().map(|process| process.pid),
                "game": game,
            }));
            text.push_str(&format!(
                "{at:>6} ms  {}{}\n",
                exe.as_deref().unwrap_or("(nada)"),
                game.map(|game| format!("  → juego: {game}"))
                    .unwrap_or_default()
            ));
            last = exe;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Report::ok(
        json!({ "ok": true, "changes": seen }),
        text.trim_end().to_owned(),
    )
}
