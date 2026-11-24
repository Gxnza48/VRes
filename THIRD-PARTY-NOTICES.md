# Avisos de terceros

VRes se distribuye bajo [GPL-3.0-or-later](LICENSE) e incluye o usa software de terceros con licencias compatibles. Las principales piezas son:

| Componente | Uso | Licencia |
|---|---|---|
| [Tauri](https://tauri.app) y sus plugins (single-instance, autostart, updater, opener) | Marco de la app de escritorio | MIT o Apache-2.0 |
| [windows-rs](https://github.com/microsoft/windows-rs) | Acceso a las APIs de Windows desde Rust | MIT o Apache-2.0 |
| [serde](https://serde.rs), [thiserror](https://github.com/dtolnay/thiserror), [clap](https://github.com/clap-rs/clap), [tokio](https://tokio.rs), [tracing](https://github.com/tokio-rs/tracing) | Utilidades de Rust | MIT o Apache-2.0 |
| [ts-rs](https://github.com/Aleph-Alpha/ts-rs) | Tipos de TypeScript generados desde Rust | MIT |
| [sys-locale](https://github.com/1Password/sys-locale) | Idioma del sistema | MIT o Apache-2.0 |
| [React](https://react.dev) | Interfaz | MIT |
| [Tailwind CSS](https://tailwindcss.com) | Estilos | MIT |
| [i18next](https://www.i18next.com) y react-i18next | Traducciones | MIT |
| [Zustand](https://github.com/pmndrs/zustand) | Estado de la interfaz | MIT |
| [Lucide](https://lucide.dev) | Íconos | ISC |
| [clsx](https://github.com/lukeed/clsx), [tailwind-merge](https://github.com/dcastil/tailwind-merge), [class-variance-authority](https://github.com/joe-bell/cva) | Utilidades de estilos | MIT / Apache-2.0 |
| Microsoft Edge WebView2 | Motor de la interfaz (se instala aparte, parte de Windows) | Licencia de Microsoft |

## Referencias técnicas

La implementación de las capas de driver se basa en documentación y SDK públicos:

- [NVIDIA NVAPI](https://github.com/NVIDIA/nvapi) (MIT).
- [AMD Display Library (ADL)](https://github.com/GPUOpen-LibrariesAndSDKs/display-library): las estructuras se declaran en Rust; no se incluyen las cabeceras del SDK.
- [Intel Graphics Control Library](https://github.com/intel/drivers.gpu.control-library): ídem.
- Documentación pública de Microsoft: Magnification API, GDI, configuración de pantalla (CCD), DDC/CI.

Si una contribución adapta código de otro proyecto con licencia compatible, se cita acá con su licencia.

La lista completa de dependencias y sus licencias se generará automáticamente antes de la versión 1.0.
