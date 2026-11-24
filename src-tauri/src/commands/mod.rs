//! Comandos que la interfaz invoca (`src/lib/ipc.ts` es el único que los llama).
//!
//! Convenciones: nombre `snake_case` con prefijo de función, siempre `async`,
//! y siempre devuelven `Result<T, IpcError>` con un código estable.

pub mod app;
pub mod crosshair;
pub mod engine;
pub mod resolution;
pub mod settings;
pub mod share;
pub mod update;
