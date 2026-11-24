//! `vres-cli`: herramienta de consola de VRes.
//!
//! Sirve para tres cosas:
//! - **restaurar** la pantalla si algo quedó mal (`vres-cli reset`);
//! - **diagnosticar** una PC (`monitors`, `diag`);
//! - **probar en hardware real** cada capa del motor y verificarla por lectura
//!   y en pantalla (`mag`, `ramp`, `probe`, `selftest`).
//!
//! Toda prueba que cambia la pantalla la restaura al terminar, aunque falle o
//! se interrumpa con `Ctrl + C`. Con `--json`, la salida es JSON en una línea.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde_json::Value;

#[cfg(windows)]
mod app;
#[cfg(windows)]
mod guard;
#[cfg(windows)]
mod hw;

/// Resultado de un comando: JSON para scripts y texto para personas.
pub struct Report {
    ok: bool,
    value: Value,
    text: String,
}

impl Report {
    pub fn ok(value: Value, text: impl Into<String>) -> Self {
        Self {
            ok: true,
            value,
            text: text.into(),
        }
    }

    pub fn fail(value: Value, text: impl Into<String>) -> Self {
        Self {
            ok: false,
            value,
            text: text.into(),
        }
    }
}

/// Tiempo máximo que una prueba mantiene un cambio en pantalla.
const MAX_HOLD_MS: u64 = 60_000;

#[derive(Debug, Parser)]
#[command(
    name = "vres-cli",
    version,
    about = "Herramienta de consola de VRes: diagnóstico, pruebas en hardware real y restauración."
)]
struct Cli {
    /// Imprime el resultado como JSON en una línea.
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
    /// Lista las pantallas: nombre, resolución, Hz, GPU y HDR.
    Monitors,
    /// Diagnóstico completo: pantallas, matriz de color y rampas.
    Diag,
    /// Matriz de color de Windows (todo el escritorio).
    #[command(subcommand)]
    Mag(MagCommand),
    /// Rampas de gamma (por pantalla).
    #[command(subcommand)]
    Ramp(RampCommand),
    /// Mediciones en hardware real.
    #[command(subcommand)]
    Probe(ProbeCommand),
    /// Aplica cada capa unos instantes, la verifica y la restaura.
    Selftest {
        /// Capas a probar, separadas por coma (mag, ramp). Por defecto, todas.
        #[arg(long, value_delimiter = ',')]
        layers: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
enum MagCommand {
    /// Muestra la matriz vigente.
    Get,
    /// Aplica una saturación, la verifica, la mantiene y la restaura.
    Set {
        /// Saturación en % (100 = sin cambio).
        #[arg(long, default_value_t = 130.0)]
        saturation: f32,
        /// Giro de tono en grados.
        #[arg(long, default_value_t = 0.0, allow_negative_numbers = true)]
        hue: f32,
        /// Cuánto tiempo mantenerla, en milisegundos.
        #[arg(long, default_value_t = 2000)]
        hold_ms: u64,
    },
    /// Vuelve la matriz a la identidad.
    Reset,
}

#[derive(Debug, Subcommand)]
enum RampCommand {
    /// Muestra la rampa vigente de cada pantalla.
    Get {
        /// Pantalla: número, `\\.\DISPLAYn` o identificador `d-…`.
        #[arg(long)]
        display: Option<String>,
        /// Incluye las 3 × 256 entradas.
        #[arg(long)]
        full: bool,
    },
    /// Aplica un ajuste fino, lo verifica, lo mantiene y restaura la original.
    Set {
        /// Pantalla: número, `\\.\DISPLAYn` o identificador `d-…`.
        #[arg(long)]
        display: String,
        /// 0,50–1,50 (mayor que 1 aclara).
        #[arg(long, default_value_t = 1.0)]
        gamma: f32,
        /// 50–170 %.
        #[arg(long, default_value_t = 100.0)]
        brightness: f32,
        /// 50–150 %.
        #[arg(long, default_value_t = 100.0)]
        contrast: f32,
        /// −100 (frío) … 100 (cálido).
        #[arg(long, default_value_t = 0.0, allow_negative_numbers = true)]
        temperature: f32,
        /// −100…100.
        #[arg(long, default_value_t = 0.0, allow_negative_numbers = true)]
        shadows: f32,
        /// −100…100.
        #[arg(long, default_value_t = 0.0, allow_negative_numbers = true)]
        highlights: f32,
        /// Cuánto tiempo mantenerla, en milisegundos.
        #[arg(long, default_value_t = 2000)]
        hold_ms: u64,
    },
    /// Escribe la identidad (también borra una calibración).
    Reset {
        /// Pantalla. Por defecto, todas.
        #[arg(long)]
        display: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum ProbeCommand {
    /// Qué tan lejos de la identidad acepta Windows una rampa.
    RampLimits {
        /// Pantalla. Por defecto, todas.
        #[arg(long)]
        display: Option<String>,
    },
    /// Cuánto tarda escribir una rampa y la matriz de color.
    Timing {
        /// Pantalla. Por defecto, todas.
        #[arg(long)]
        display: Option<String>,
        /// Cantidad de escrituras.
        #[arg(long, default_value_t = 30)]
        samples: usize,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let report = run(cli.command);
    if cli.json {
        println!("{}", report.value);
    } else {
        println!("{}", report.text);
    }
    if report.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(windows)]
fn run(command: Command) -> Report {
    use std::time::Duration;

    vres_win::dpi::make_process_per_monitor_aware();
    guard::install_interrupt_handler();
    let hold = |ms: u64| Duration::from_millis(ms.min(MAX_HOLD_MS));

    match command {
        Command::Status => app::status(),
        Command::Reset => app::reset(),
        Command::Shutdown { timeout_ms } => app::shutdown(Duration::from_millis(timeout_ms)),
        Command::Monitors => hw::monitors(),
        Command::Diag => hw::diag(),
        Command::Mag(MagCommand::Get) => hw::mag_get(),
        Command::Mag(MagCommand::Set {
            saturation,
            hue,
            hold_ms,
        }) => hw::mag_set(saturation, hue, hold(hold_ms)),
        Command::Mag(MagCommand::Reset) => hw::mag_reset(),
        Command::Ramp(RampCommand::Get { display, full }) => hw::ramp_get(display.as_deref(), full),
        Command::Ramp(RampCommand::Set {
            display,
            gamma,
            brightness,
            contrast,
            temperature,
            shadows,
            highlights,
            hold_ms,
        }) => hw::ramp_set(
            &display,
            hw::ToneArgs {
                gamma,
                brightness,
                contrast,
                temperature,
                shadows,
                highlights,
            },
            hold(hold_ms),
        ),
        Command::Ramp(RampCommand::Reset { display }) => hw::ramp_reset(display.as_deref()),
        Command::Probe(ProbeCommand::RampLimits { display }) => {
            hw::probe_ramp_limits(display.as_deref())
        }
        Command::Probe(ProbeCommand::Timing { display, samples }) => {
            hw::probe_timing(display.as_deref(), samples.clamp(2, 500))
        }
        Command::Selftest { layers } => hw::selftest(&layers),
    }
}

#[cfg(not(windows))]
fn run(_command: Command) -> Report {
    Report::fail(
        serde_json::json!({ "ok": false }),
        "vres-cli solo funciona en Windows.",
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

    #[test]
    fn parses_negative_temperature() {
        let cli = Cli::try_parse_from([
            "vres-cli",
            "ramp",
            "set",
            "--display",
            "1",
            "--temperature",
            "-40",
        ])
        .expect("argumentos válidos");
        assert!(matches!(
            cli.command,
            Command::Ramp(RampCommand::Set { temperature, .. }) if temperature == -40.0
        ));
    }

    #[test]
    fn parses_selftest_layers() {
        let cli = Cli::try_parse_from(["vres-cli", "selftest", "--layers", "mag,ramp"])
            .expect("argumentos válidos");
        assert!(matches!(cli.command, Command::Selftest { layers } if layers == ["mag", "ramp"]));
    }
}
