//! Persistencia en JSON: escritura atómica y cuarentena de archivos rotos.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde::de::DeserializeOwned;

/// Escribe `value` como JSON de forma atómica: primero a un archivo temporal
/// (con `sync_all`) y después lo reemplaza. Si se corta la luz en el medio,
/// queda el archivo viejo o el nuevo, nunca uno a medias.
pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let mut json = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    json.push(b'\n');
    write_bytes_atomic(path, &json)
}

/// Escribe bytes de forma atómica (temporal con `sync_all` y reemplazo).
pub fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let temp = path.with_extension("tmp");
    {
        let mut file = fs::File::create(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    // Windows puede tener el archivo abierto por un instante (antivirus,
    // indexador): se reintenta unas veces.
    let mut last_error = None;
    for _ in 0..5 {
        match fs::rename(&temp, path) {
            Ok(()) => return Ok(()),
            Err(error) => {
                last_error = Some(error);
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
    }
    Err(last_error.unwrap_or_else(|| io::Error::other("rename failed")))
}

/// Resultado de leer un archivo JSON.
#[derive(Debug)]
pub enum Loaded<T> {
    /// No existe.
    Missing,
    /// Se leyó bien.
    Ok(T),
    /// Estaba roto: se movió a `quarantined` para no perderlo.
    Corrupt { quarantined: Option<PathBuf> },
}

/// Lee un JSON. Si está roto, lo renombra a `*.corrupt-<segundos>` y
/// devuelve [`Loaded::Corrupt`] (VRes sigue con los valores por defecto).
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Loaded<T> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Loaded::Missing,
        Err(_) => return Loaded::Corrupt { quarantined: None },
    };
    match serde_json::from_slice(&bytes) {
        Ok(value) => Loaded::Ok(value),
        Err(_) => {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or_default();
            let quarantined = path.with_extension(format!("corrupt-{stamp}"));
            let moved = fs::rename(path, &quarantined).is_ok();
            Loaded::Corrupt {
                quarantined: moved.then_some(quarantined),
            }
        }
    }
}

/// Bytes a texto hexadecimal (para guardar rampas en JSON).
pub fn to_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[usize::from(byte >> 4)] as char);
        out.push(DIGITS[usize::from(byte & 0x0f)] as char);
    }
    out
}

/// Texto hexadecimal a bytes. `None` si no es válido.
pub fn from_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    let nibble = |c: u8| match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    };
    text.as_bytes()
        .chunks(2)
        .map(|pair| Some((nibble(pair[0])? << 4) | nibble(pair[1])?))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Sample {
        value: u32,
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("vres-store-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn atomic_write_then_read() {
        let dir = temp_dir("roundtrip");
        let path = dir.join("state.json");
        write_json_atomic(&path, &Sample { value: 7 }).expect("escritura");
        write_json_atomic(&path, &Sample { value: 8 }).expect("reemplazo");
        assert!(matches!(
            read_json::<Sample>(&path),
            Loaded::Ok(Sample { value: 8 })
        ));
        assert!(!path.with_extension("tmp").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_and_corrupt_files() {
        let dir = temp_dir("corrupt");
        let path = dir.join("state.json");
        assert!(matches!(read_json::<Sample>(&path), Loaded::Missing));
        fs::create_dir_all(&dir).expect("carpeta");
        fs::write(&path, b"{ roto").expect("archivo roto");
        match read_json::<Sample>(&path) {
            Loaded::Corrupt {
                quarantined: Some(quarantined),
            } => assert!(quarantined.exists()),
            other => panic!("se esperaba Corrupt: {other:?}"),
        }
        assert!(!path.exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn hex_round_trip() {
        let bytes = [0u8, 1, 0x7f, 0x80, 0xff];
        assert_eq!(to_hex(&bytes), "00017f80ff");
        assert_eq!(from_hex("00017F80ff").as_deref(), Some(&bytes[..]));
        assert_eq!(from_hex("0"), None);
        assert_eq!(from_hex("zz"), None);
    }
}
