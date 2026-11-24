//! # vres-engine
//!
//! El motor de VRes (fase F1b). Un único hilo de sistema, dueño de una ventana
//! oculta y de su bucle de mensajes, que:
//!
//! 1. recibe lo que pide el usuario (`Requested`);
//! 2. le pide al planificador de `vres-core` un `Plan` (función pura);
//! 3. escribe solo lo que cambió, lo **relee** para confirmarlo y publica el
//!    estado de cada capa (matriz, rampas, vibrance, zoom);
//! 4. guarda los valores originales antes de tocar nada y los restaura al
//!    salir, ante un fallo o al próximo arranque.
//!
//! No depende de Tauri ni de tokio: se prueba desde `vres-cli` y con un
//! hardware falso en los tests.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]

/// Fase del plan en la que el motor queda operativo.
pub const AVAILABLE_IN_PHASE: &str = "F1";

#[cfg(test)]
mod tests {
    #[test]
    fn phase_is_declared() {
        assert_eq!(super::AVAILABLE_IN_PHASE, "F1");
    }
}
