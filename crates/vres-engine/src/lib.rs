//! # vres-engine
//!
//! El motor de VRes. Un único hilo de sistema, dueño de una ventana oculta y
//! de su bucle de mensajes, que:
//!
//! 1. recibe lo que pide el usuario (`Requested`);
//! 2. le pide al planificador de `vres-core` un `Plan` (función pura);
//! 3. escribe solo lo que cambió, lo **relee** para confirmarlo y publica el
//!    estado de cada capa;
//! 4. guarda los valores originales antes de tocar nada y los restaura al
//!    salir, ante un fallo o al próximo arranque.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]

pub mod core;
pub mod resolution;

#[cfg(windows)]
mod thread;

#[cfg(windows)]
pub use thread::{
    ApplyMode, Command, EngineConfig, EngineEvent, EngineHandle, EngineSettings, EngineSnapshot,
    EventSink, HotkeyStatus, Reply, emergency_restore, modes_restore_path, recover_from_disk,
    restore_path, spawn, state_dir,
};
