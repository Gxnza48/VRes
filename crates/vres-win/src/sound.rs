//! Tonos cortos para confirmar un atajo sin mirar la pantalla.
//!
//! Se generan en memoria (WAV de 16 bits) la primera vez: no hay archivos de
//! audio en el instalador.

use std::sync::OnceLock;

use windows::Win32::Media::Audio::{PlaySoundW, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};
use windows::core::PCWSTR;

/// Qué tono sonar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Ascendente: se activó algo (por ejemplo, la resolución estirada).
    Up,
    /// Descendente: se volvió a lo normal.
    Down,
}

const SAMPLE_RATE: u32 = 22_050;
const NOTE_MS: u32 = 70;
const VOLUME: f32 = 0.22;

/// WAV PCM mono con una nota por frecuencia, con rampas cortas para que no
/// haga clic.
fn wav(frequencies: &[f32]) -> Vec<u8> {
    let per_note = (SAMPLE_RATE * NOTE_MS / 1000) as usize;
    let fade = per_note / 8;
    let mut samples = Vec::with_capacity(per_note * frequencies.len());
    for &frequency in frequencies {
        for i in 0..per_note {
            let t = i as f32 / SAMPLE_RATE as f32;
            let envelope = if i < fade {
                i as f32 / fade as f32
            } else if i > per_note - fade {
                (per_note - i) as f32 / fade as f32
            } else {
                1.0
            };
            let value = (t * frequency * std::f32::consts::TAU).sin() * envelope * VOLUME;
            samples.push((value * f32::from(i16::MAX)) as i16);
        }
    }

    let data_len = (samples.len() * 2) as u32;
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes()); // tamaño del bloque fmt
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // bytes por segundo
    bytes.extend_from_slice(&2u16.to_le_bytes()); // alineación
    bytes.extend_from_slice(&16u16.to_le_bytes()); // bits por muestra
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

static UP: OnceLock<Vec<u8>> = OnceLock::new();
static DOWN: OnceLock<Vec<u8>> = OnceLock::new();

/// Suena el tono sin bloquear.
pub fn play(tone: Tone) {
    let bytes = match tone {
        Tone::Up => UP.get_or_init(|| wav(&[660.0, 990.0])),
        Tone::Down => DOWN.get_or_init(|| wav(&[990.0, 660.0])),
    };
    // SAFETY: con SND_MEMORY el primer parámetro apunta a la imagen WAV en
    // memoria; es `'static`, así que sigue viva mientras suena (SND_ASYNC).
    let _ = unsafe {
        PlaySoundW(
            PCWSTR(bytes.as_ptr().cast()),
            None,
            SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_matches_the_data() {
        let bytes = wav(&[440.0, 880.0]);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        let data_len = u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]) as usize;
        assert_eq!(bytes.len(), 44 + data_len);
        let samples_per_note = (SAMPLE_RATE * NOTE_MS / 1000) as usize;
        assert_eq!(data_len, samples_per_note * 2 * 2);
    }
}
