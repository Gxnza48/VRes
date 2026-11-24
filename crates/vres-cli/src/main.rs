//! `vres-cli`: herramienta de consola de VRes.
//!
//! Sirve para tres cosas:
//! - **restaurar** la pantalla si algo quedó mal (`vres-cli reset`);
//! - **diagnosticar** una PC (monitores, GPU, capacidades) — desde F1;
//! - **probar en hardware real** cada capa del motor y verificarla por lectura
//!   (`selftest`, `probe`) — desde F1.
//!
//! Con `--json` toda la salida es JSON en una línea, para scripts y CI.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]

use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, Subcommand};
use serde_json::json;

#[derive(Debug, Parser)]
#[command(
    name = "vres-cli",
    version,
    about = "Herramienta de consola de VRes: diagnóstico, pruebas en hardware real y restauración."
)]
struct Cli {
    /// Imprime el resultado como JSON.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Indica si VRes está corriendo y si responde.
    Status,
    /// Vuelve la pantalla a la normalidad (le pide a VRes que restaure todo).
    Reset,
    /// Cierra VRes restaurando todo y espera a que termine.
    Shutdown {
        /// Tiempo máximo de espera, en milisegundos.
        #[arg(long, default_value_t = 3000)]
        timeout_ms: u64,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let (ok, report) = run(&cli.command);
    if cli.json {
        println!("{report}");
    } else if let Some(text) = report.get("message").and_then(|m| m.as_str()) {
        println!("{text}");
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(windows)]
fn run(command: &Command) -> (bool, serde_json::Value) {
    use vres_win::control::{self, ACK, RemoteCommand, ShutdownOutcome};

    const SEND_TIMEOUT: Duration = Duration::from_secs(2);

    match command {
        Command::Status => match control::send(RemoteCommand::Ping, SEND_TIMEOUT) {
            Ok(Some(answer)) => (
                true,
                json!({ "ok": true, "running": true, "responding": answer == ACK,
                        "message": "VRes está corriendo y responde." }),
            ),
            Ok(None) => (
                true,
                json!({ "ok": true, "running": false, "message": "VRes no está corriendo." }),
            ),
            Err(error) => (
                false,
                json!({ "ok": false, "running": true, "responding": false,
                        "error": error.to_string(),
                        "message": "VRes está corriendo pero no responde." }),
            ),
        },
        Command::Reset => match control::send(RemoteCommand::EmergencyReset, SEND_TIMEOUT) {
            Ok(Some(_)) => (
                true,
                json!({ "ok": true, "running": true,
                        "message": "Listo: VRes restableció la pantalla." }),
            ),
            // Sin instancia en ejecución no hay nada aplicado por la sesión
            // actual. La restauración desde `restore.json` llega en F1.
            Ok(None) => (
                true,
                json!({ "ok": true, "running": false,
                        "message": "VRes no está corriendo: no hay cambios activos que restablecer." }),
            ),
            Err(error) => (
                false,
                json!({ "ok": false, "error": error.to_string(),
                        "message": "No se pudo contactar a VRes." }),
            ),
        },
        Command::Shutdown { timeout_ms } => {
            let started = std::time::Instant::now();
            match control::shutdown_running(Duration::from_millis(*timeout_ms)) {
                Ok(outcome) => {
                    let elapsed_ms = started.elapsed().as_millis();
                    let (ok, message) = match outcome {
                        ShutdownOutcome::NotRunning => (true, "VRes no está corriendo."),
                        ShutdownOutcome::Exited => (true, "VRes se cerró."),
                        ShutdownOutcome::StillRunning => {
                            (false, "VRes no terminó dentro del tiempo de espera.")
                        }
                    };
                    (
                        ok,
                        json!({ "ok": ok, "outcome": format!("{outcome:?}"),
                                "elapsedMs": elapsed_ms, "message": message }),
                    )
                }
                Err(error) => (
                    false,
                    json!({ "ok": false, "error": error.to_string(),
                            "message": "No se pudo contactar a VRes." }),
                ),
            }
        }
    }
}

#[cfg(not(windows))]
fn run(_command: &Command) -> (bool, serde_json::Value) {
    (
        false,
        json!({ "ok": false, "message": "vres-cli solo funciona en Windows." }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_shutdown_timeout() {
        let cli = Cli::try_parse_from(["vres-cli", "--json", "shutdown", "--timeout-ms", "500"])
            .expect("argumentos válidos");
        assert!(cli.json);
        assert!(matches!(cli.command, Command::Shutdown { timeout_ms: 500 }));
    }
}
