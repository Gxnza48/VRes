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
mod crosshair;
#[cfg(windows)]
mod guard;
#[cfg(windows)]
mod hw;
#[cfg(windows)]
mod resolution;
mod share;
#[cfg(windows)]
mod vibrance;

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
    /// Resolución: modos de cada pantalla, prueba y cambio temporal.
    #[command(subcommand)]
    Mode(ModeCommand),
    /// HDR de cada pantalla.
    #[command(subcommand)]
    Hdr(HdrCommand),
    /// Vibrance del driver (NVIDIA y AMD), por pantalla.
    #[command(subcommand)]
    Vibrance(VibranceCommand),
    /// Ventana del crosshair.
    #[command(subcommand)]
    Overlay(OverlayCommand),
    /// Códigos para compartir (looks, crosshairs y resoluciones).
    #[command(subcommand)]
    Share(ShareCommand),
    /// Mediciones en hardware real.
    #[command(subcommand)]
    Probe(ProbeCommand),
    /// Aplica cada capa unos instantes, la verifica y la restaura.
    Selftest {
        /// Capas a probar, separadas por coma (mag, ramp). Por defecto, todas.
        #[arg(long, value_delimiter = ',')]
        layers: Vec<String>,
    },
    /// Uso interno de `probe mode-kill-revert`.
    #[command(hide = true)]
    InternalModeHold {
        #[arg(long)]
        display: String,
        #[arg(long)]
        mode: String,
        #[arg(long, default_value = "stretch")]
        scaling: String,
    },
}

#[derive(Debug, Subcommand)]
enum ModeCommand {
    /// Lista los modos de cada pantalla, el actual, el de Windows y la nativa.
    List {
        /// Pantalla: número, `\\.\DISPLAYn` o identificador `d-…`.
        #[arg(long)]
        display: Option<String>,
    },
    /// Pregunta si un modo se puede aplicar, sin cambiar nada.
    Test {
        #[arg(long)]
        display: String,
        /// `1440x1080` o `1440x1080@200`.
        mode: String,
        /// stretch, aspect, center o preferred.
        #[arg(long, default_value = "stretch")]
        scaling: String,
    },
    /// Aplica un modo temporal, relee el escalado, lo mantiene y vuelve.
    Set {
        #[arg(long)]
        display: String,
        /// `1440x1080` o `1440x1080@200`.
        mode: String,
        /// stretch, aspect, center o preferred.
        #[arg(long, default_value = "stretch")]
        scaling: String,
        /// Cuánto tiempo mantenerlo, en milisegundos (máximo 10 000).
        #[arg(long, default_value_t = 3000)]
        hold_ms: u64,
        /// Confirma que la pantalla puede parpadear y cambiar unos segundos.
        #[arg(long)]
        allow_mode_switch: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ShareCommand {
    /// Muestra qué hay en un código.
    Decode {
        /// Por ejemplo `VRL1-…`.
        code: String,
    },
    /// Arma el código de un look o de un crosshair (en JSON).
    Encode {
        #[arg(long)]
        look: Option<String>,
        #[arg(long)]
        crosshair: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
enum OverlayCommand {
    /// Dibuja un píxel de prueba en el centro de cada pantalla y verifica el centrado.
    Test {
        /// Pantalla. Por defecto, todas.
        #[arg(long)]
        display: Option<String>,
        /// Cuánto tiempo mantenerlo, en milisegundos.
        #[arg(long, default_value_t = 500)]
        hold_ms: u64,
    },
}

#[derive(Debug, Subcommand)]
enum VibranceCommand {
    /// Rango y valor actual de cada pantalla.
    Get {
        /// Pantalla: número, `\.DISPLAYn` o identificador `d-…`.
        #[arg(long)]
        display: Option<String>,
    },
    /// Aplica un valor, lo relee, lo mantiene y vuelve al original.
    Set {
        #[arg(long)]
        display: String,
        /// Escala de VRes: 0–100 (50 = normal del driver).
        #[arg(long, default_value_t = 70.0)]
        value: f32,
        /// Cuánto tiempo mantenerlo, en milisegundos.
        #[arg(long, default_value_t = 2000)]
        hold_ms: u64,
    },
}

#[derive(Debug, Subcommand)]
enum HdrCommand {
    /// Estado de HDR de cada pantalla.
    Get,
    /// Cambia el HDR, lo verifica, lo mantiene y lo deja como estaba.
    Set {
        #[arg(long)]
        display: String,
        /// on u off.
        state: String,
        /// Cuánto tiempo mantenerlo, en milisegundos (máximo 10 000).
        #[arg(long, default_value_t = 3000)]
        hold_ms: u64,
        /// Confirma que la pantalla puede parpadear unos segundos.
        #[arg(long)]
        allow_mode_switch: bool,
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
    /// El motor real aplica la vibrance del driver y la devuelve al apagarse.
    EngineVibrance {
        /// Escala de VRes: 0–100 (50 = normal del driver).
        #[arg(long, default_value_t = 30.0)]
        value: f32,
        /// Cuánto tiempo mantenerla, en milisegundos.
        #[arg(long, default_value_t = 1500)]
        hold_ms: u64,
    },
    /// El motor real aplica, deja vencer, conserva y restaura un modo.
    EngineModes {
        #[arg(long)]
        display: String,
        /// `1440x1080` o `1440x1080@200`.
        mode: String,
        /// Confirma que la pantalla puede parpadear y cambiar varias veces.
        #[arg(long)]
        allow_mode_switch: bool,
    },
    /// ¿Windows revierte solo un modo temporal si el proceso que lo aplicó muere?
    ModeKillRevert {
        #[arg(long)]
        display: String,
        /// `1440x1080` o `1440x1080@200`.
        mode: String,
        #[arg(long, default_value = "stretch")]
        scaling: String,
        /// Confirma que la pantalla puede parpadear y cambiar unos segundos.
        #[arg(long)]
        allow_mode_switch: bool,
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
        Command::Probe(ProbeCommand::EngineVibrance { value, hold_ms }) => {
            vibrance::probe_engine(value.clamp(0.0, 100.0), hold(hold_ms))
        }
        Command::Probe(ProbeCommand::EngineModes {
            display,
            mode,
            allow_mode_switch,
        }) => resolution::probe_engine_modes(&display, &mode, allow_mode_switch),
        Command::Probe(ProbeCommand::ModeKillRevert {
            display,
            mode,
            scaling,
            allow_mode_switch,
        }) => resolution::probe_mode_kill_revert(&display, &mode, &scaling, allow_mode_switch),
        Command::Mode(ModeCommand::List { display }) => resolution::mode_list(display.as_deref()),
        Command::Mode(ModeCommand::Test {
            display,
            mode,
            scaling,
        }) => resolution::mode_test(&display, &mode, &scaling),
        Command::Mode(ModeCommand::Set {
            display,
            mode,
            scaling,
            hold_ms,
            allow_mode_switch,
        }) => resolution::mode_set(
            &display,
            &mode,
            &scaling,
            Duration::from_millis(hold_ms.min(resolution::MAX_MODE_HOLD_MS)),
            allow_mode_switch,
        ),
        Command::Share(ShareCommand::Decode { code }) => share::decode(&code),
        Command::Share(ShareCommand::Encode { look, crosshair }) => {
            share::encode(look.as_deref(), crosshair.as_deref())
        }
        Command::Overlay(OverlayCommand::Test { display, hold_ms }) => {
            crosshair::overlay_test(display.as_deref(), hold(hold_ms))
        }
        Command::Vibrance(VibranceCommand::Get { display }) => vibrance::get(display.as_deref()),
        Command::Vibrance(VibranceCommand::Set {
            display,
            value,
            hold_ms,
        }) => vibrance::set(&display, value.clamp(0.0, 100.0), hold(hold_ms)),
        Command::Hdr(HdrCommand::Get) => resolution::hdr_get(),
        Command::Hdr(HdrCommand::Set {
            display,
            state,
            hold_ms,
            allow_mode_switch,
        }) => match state.as_str() {
            "on" | "off" => resolution::hdr_set(
                &display,
                state == "on",
                Duration::from_millis(hold_ms.min(resolution::MAX_MODE_HOLD_MS)),
                allow_mode_switch,
            ),
            _ => Report::fail(
                serde_json::json!({ "ok": false, "error": "state must be on or off" }),
                "El estado tiene que ser on u off.",
            ),
        },
        Command::InternalModeHold {
            display,
            mode,
            scaling,
        } => resolution::mode_hold(&display, &mode, &scaling),
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
