//! Respaldo del potato para drivers viejos de NVIDIA (rama anterior a 500),
//! que pueden ignorar el LOD Bias en DirectX 11: baja el límite de texturas de
//! Unity en el archivo de ajustes del juego, el mismo que escribe su menú de
//! gráficos. Solo se toca con el juego cerrado, cambiando ese único número, y
//! VRes recuerda el valor del jugador para devolverlo al elegir «Normal».

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use vres_core::potato::{MIPMAP_KEY, PotatoLevel, mipmap_limit, read_int, write_int};

/// Carpeta de ajustes del juego dentro de `AppData\LocalLow`.
fn settings_dir(game_id: &str) -> Option<PathBuf> {
    let relative = match game_id {
        "robrun" => ["Kefir", "ROB RUN", "settings"],
        _ => return None,
    };
    let mut dir = PathBuf::from(std::env::var_os("USERPROFILE")?);
    dir.push("AppData");
    dir.push("LocalLow");
    for part in relative {
        dir.push(part);
    }
    Some(dir)
}

/// Archivos de ajustes del juego (uno por cuenta de Steam).
fn settings_files(game_id: &str) -> Vec<PathBuf> {
    let Some(dir) = settings_dir(game_id) else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("settings_") && name.ends_with(".dat"))
        })
        .collect();
    files.sort();
    files
}

/// Valores originales del jugador, por archivo.
fn originals_path() -> Option<PathBuf> {
    vres_engine::state_dir().map(|dir| dir.join("potato-originals.json"))
}

fn load_originals(path: &Path) -> BTreeMap<String, u32> {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Escribe en un temporal y lo reemplaza, para no dejar el archivo a medias.
fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    let temporary = path.with_extension("vres-tmp");
    fs::write(&temporary, text)?;
    fs::rename(&temporary, path)
}

/// Pone el límite de texturas del nivel en todos los archivos del juego.
/// Sin archivos (el juego nunca se abrió) no hay nada que hacer.
pub fn apply(game_id: &str, level: PotatoLevel) -> Result<(), String> {
    let files = settings_files(game_id);
    if files.is_empty() {
        return Ok(());
    }
    let originals_file = originals_path().ok_or("LOCALAPPDATA is not set")?;
    apply_to(&files, &originals_file, level)
}

fn apply_to(files: &[PathBuf], originals_file: &Path, level: PotatoLevel) -> Result<(), String> {
    let mut originals = load_originals(originals_file);
    for file in files {
        let key = file.to_string_lossy().into_owned();
        let text = fs::read_to_string(file).map_err(|error| format!("{key}: {error}"))?;
        let Some(current) = read_int(&text, MIPMAP_KEY) else {
            continue;
        };
        let wanted = match mipmap_limit(level) {
            Some(limit) => {
                originals.entry(key.clone()).or_insert(current);
                limit
            }
            None => match originals.remove(&key) {
                Some(original) => original,
                None => continue,
            },
        };
        if wanted != current {
            let edited = write_int(&text, MIPMAP_KEY, wanted).ok_or("mipmap key vanished")?;
            write_atomic(file, &edited).map_err(|error| format!("{key}: {error}"))?;
        }
    }
    if let Some(dir) = originals_file.parent() {
        fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    }
    let json = serde_json::to_string_pretty(&originals).map_err(|error| error.to_string())?;
    write_atomic(originals_file, &json).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn potato_then_normal_restores_the_player_value() {
        let dir = std::env::temp_dir().join(format!("vres-potato-file-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings_steam_1_2.dat");
        let originals = dir.join("state").join("potato-originals.json");
        let before =
            r#"{"render_scale_setting":0.800000012,"global_texture_mipmap_limit":1,"x":true}"#;
        fs::write(&file, before).unwrap();
        let files = vec![file.clone()];

        apply_to(&files, &originals, PotatoLevel::Potato).unwrap();
        let potato = fs::read_to_string(&file).unwrap();
        assert_eq!(read_int(&potato, MIPMAP_KEY), Some(15));
        assert!(potato.contains("0.800000012"));

        apply_to(&files, &originals, PotatoLevel::Low).unwrap();
        assert_eq!(
            read_int(&fs::read_to_string(&file).unwrap(), MIPMAP_KEY),
            Some(3)
        );

        apply_to(&files, &originals, PotatoLevel::Off).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), before);

        // «Normal» sin nada guardado no toca el archivo.
        apply_to(&files, &originals, PotatoLevel::Off).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), before);
        fs::remove_dir_all(&dir).unwrap();
    }
}
