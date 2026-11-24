// En release, VRes es una app de ventana: sin consola. NO QUITAR.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![forbid(unsafe_code)]

fn main() {
    // `--version`, `--shutdown` y `--reset` se atienden antes de arrancar Tauri:
    // los usan el instalador y los accesos directos, y tienen que ser rápidos.
    if let Some(code) = vres_lib::handle_early_flags() {
        std::process::exit(code);
    }
    vres_lib::run();
}
