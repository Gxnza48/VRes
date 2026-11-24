//! Errores de las llamadas a Windows y a los drivers.

/// Error de una llamada a Windows o a un driver.
///
/// Los mensajes están en inglés porque son técnicos: van a los logs y a los
/// reportes de diagnóstico. La interfaz traduce por código, no por mensaje.
#[derive(Debug, thiserror::Error)]
pub enum WinError {
    /// Una función de Win32 devolvió error.
    #[error("{api} failed: {source}")]
    Api {
        /// Nombre de la función.
        api: &'static str,
        /// Error de Windows (incluye el código HRESULT).
        #[source]
        source: windows::core::Error,
    },
    /// La operación no está soportada en esta PC.
    #[error("unsupported: {0}")]
    Unsupported(&'static str),
    /// Se agotó el tiempo de espera.
    #[error("timed out: {0}")]
    Timeout(&'static str),
    /// Un hilo auxiliar terminó antes de responder.
    #[error("worker thread stopped: {0}")]
    ThreadStopped(&'static str),
    /// Una llamada COM devolvió error (solo el código: el error de Windows
    /// guarda objetos COM que no pueden sobrevivir al apartamento).
    #[error("{api} failed: HRESULT 0x{code:08X}")]
    Com {
        /// Nombre de la llamada.
        api: &'static str,
        /// HRESULT.
        code: u32,
    },
    /// Un cambio de pantalla devolvió un código de error (`DISP_CHANGE_*` o
    /// `ERROR_*`).
    #[error("{api} returned {code}")]
    DisplayChange {
        /// Nombre de la función.
        api: &'static str,
        /// Código devuelto.
        code: i32,
    },
}

/// Resultado de las funciones de este crate.
pub type WinResult<T> = Result<T, WinError>;

impl WinError {
    /// Envuelve el último error de Win32 (`GetLastError`) de la función `api`.
    pub(crate) fn last(api: &'static str) -> Self {
        Self::Api {
            api,
            source: windows::core::Error::from_thread(),
        }
    }

    /// Error de una llamada COM, guardando solo su HRESULT.
    pub(crate) fn com(api: &'static str, source: &windows::core::Error) -> Self {
        Self::Com {
            api,
            code: source.code().0 as u32,
        }
    }

    /// Envuelve un error de Win32 devuelto por la función `api`.
    pub(crate) fn api(api: &'static str, source: windows::core::Error) -> Self {
        Self::Api { api, source }
    }
}
