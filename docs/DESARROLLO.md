# Desarrollo

## Requisitos

- Windows 10/11 x64
- [Node.js](https://nodejs.org) 22 o superior (npm)
- [Rust](https://rustup.rs) estable para MSVC (el repo fija `stable` con `rustfmt` y `clippy` en `rust-toolchain.toml`)
- Visual Studio Build Tools 2022 con "Desarrollo para el escritorio con C++"
- WebView2 (Windows 11 ya lo trae)

## Primeros pasos

```powershell
git clone https://github.com/Gxnza48/VRes.git
cd VRes
powershell -ExecutionPolicy Bypass -File scripts\setup-dev.ps1
npm ci
npm run tauri dev
```

`scripts\setup-dev.ps1` genera `.cargo\config.local.toml` (no se versiona) para que Cargo compile en `%LOCALAPPDATA%\vres-dev\target`. Es especialmente importante si el repo vive dentro de **OneDrive**: la compilación de Rust genera varios GB que OneDrive intentaría sincronizar (y bloquear).

## Comandos

| Qué | Comando |
|---|---|
| App en modo desarrollo | `npm run tauri dev` |
| Compilar la app sin instalador | `npm run tauri build -- --debug --no-bundle` |
| Instalador (requiere la clave de firma del updater) | `npm run tauri build` |
| Tests de Rust (también regeneran `src/lib/bindings`) | `cargo test --workspace` |
| Formato y lint de Rust | `cargo fmt --all` · `cargo clippy --workspace --all-targets -- -D warnings` |
| Tests de la interfaz | `npx vitest --run` |
| Lint y tipos | `npm run lint` · `npm run typecheck` |
| Traducciones completas | `npm run check:i18n` |
| Colores solo desde los tokens | `npm run check:tokens` |
| Versión sincronizada | `npm run check:version` |
| Herramienta de consola | `cargo run -p vres-cli -- --help` |

`cargo clippy` y `cargo test` del workspace compilan la app de Tauri, que embebe `dist/`: corre `npm run build` antes si todavía no existe.

## Estructura

```
crates/vres-core     lógica pura (sin Windows ni Tauri): modelos, color, planificador, IPC
crates/vres-win      acceso a Windows y a los drivers (único crate con `unsafe`)
crates/vres-engine   el motor (hilo propio, sin Tauri)
crates/vres-cli      herramienta de consola
src-tauri/           la app de Tauri (comandos, bandeja, ventanas, updater)
src/                 la interfaz (React + TypeScript + Tailwind)
  lib/ipc.ts         único punto de llamada al motor
  lib/bindings/      tipos generados desde Rust (no editar)
  styles/tokens.css  todos los colores y medidas de la interfaz
  locales/           textos en es / en / pt-BR (el español es la fuente)
docs/                documentación
scripts/             verificaciones y utilidades
```

## Reglas

- **Sala limpia:** no se copia código, presets, textos ni recursos de otros productos (ver [CONTRIBUTING.md](../CONTRIBUTING.md)).
- `unsafe` solo en `vres-win`, y cada bloque con su comentario `// SAFETY:`.
- Nada de hooks de teclado o mouse de bajo nivel, de abrir el proceso de un juego ni de conexiones de red nuevas.
- Comandos de Tauri siempre `async`; ventanas creadas desde tareas async.
- Ningún color literal fuera de `src/styles` y `src/components/ui`.
- Todo texto visible, en los tres idiomas.

## Pruebas en hardware real

Las pruebas que tocan la pantalla **siempre restauran**: usan un guardián que deshace los cambios al terminar, incluso ante un error o `Ctrl + C`, y tienen un tiempo máximo. Usan valores suaves y breves (nada de parpadeos ni inversión de colores). Los cambios de resolución exigen `--allow-mode-switch`, usan solo modos que el monitor informa y vuelven solos en 3 segundos.

Si algo quedó mal: `cargo run -p vres-cli -- reset` (o `ramp reset` / `mag reset`).

| Comando | Qué hace | ¿Cambia la pantalla? |
|---|---|---|
| `vres-cli monitors` | Lista las pantallas: nombre, modo, GPU, HDR | No |
| `vres-cli diag` | Pantallas, estado de la matriz y de cada rampa | No |
| `vres-cli selftest` | Saturación 130 % y gamma 1,05 durante 0,3 s; verifica por lectura y en pantalla; restaura | Sí, suave y breve |
| `vres-cli mag set --saturation 150 --hold-ms 2000` | Aplica una saturación, la mantiene y la restaura | Sí |
| `vres-cli ramp set --display 1 --gamma 1.2` | Aplica un ajuste fino a una pantalla y restaura la original | Sí |
| `vres-cli probe timing` | Mide cuánto tarda escribir rampas y la matriz (cambios mínimos) | Casi imperceptible |
| `vres-cli probe ramp-limits` | Busca el límite de las rampas: **aclara la pantalla un instante varias veces**, con pausas de 0,6 s | Sí, visible |

Todos aceptan `--json`. Las pantallas se eligen con `--display` por número (`1`), nombre GDI (`\\.\DISPLAY1`) o identificador estable (`d-…`).

Los resultados se registran en [hardware/RESULTADOS.md](hardware/RESULTADOS.md).

## Publicar una versión

1. Actualizar la versión en `Cargo.toml` (`[workspace.package]`) y en `package.json` (`npm run check:version` verifica que coincidan).
2. Actualizar `CHANGELOG.md` y `docs/ROADMAP.md`.
3. Commit y tag: `git tag vX.Y.Z && git push --tags`.
4. El workflow `Release` compila, firma y publica el instalador, `latest.json` (para las actualizaciones automáticas), `vres-cli.exe` y `SHA256SUMS.txt`.

La clave privada que firma las actualizaciones vive **fuera del repo** (en la PC del mantenedor y en los secretos de GitHub `TAURI_SIGNING_PRIVATE_KEY` y `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`). Si se pierde, las instalaciones existentes dejan de recibir actualizaciones.
