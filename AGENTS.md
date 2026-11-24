# Instrucciones para agentes de IA

Este archivo es para cualquier asistente de IA (Claude, Codex, Copilot…) que trabaje en VRes. Las personas también pueden leerlo: resume las reglas del proyecto.

## Qué es VRes

App de bandeja para Windows 10/11 (Tauri 2 + Rust + React/TypeScript) que ajusta cómo el monitor muestra la imagen: saturación, vibrance por driver, ajuste fino por monitor, resolución estirada, crosshair, perfiles por juego, Dark Boost, zoom y control desde el celular. Gratis, GPL-3.0, sin cuenta ni telemetría. Se construye por fases ([docs/ROADMAP.md](docs/ROADMAP.md)); la arquitectura está en [docs/ARQUITECTURA.md](docs/ARQUITECTURA.md).

## Reglas (no negociables)

1. **Sala limpia.** No abrir, copiar ni adaptar código, presets, tablas de valores, textos, nombres ni recursos de otros productos de ajuste de pantalla o de sus clones. Solo documentación pública de Microsoft y de los fabricantes, matemática de color estándar y código con licencia compatible (citado en `THIRD-PARTY-NOTICES.md`).
2. **Nunca dejar la pantalla modificada.** Toda prueba que toca la pantalla restaura al terminar (guardián + tiempo máximo). Después de cualquier fallo: `cargo run -p vres-cli -- reset`.
3. **`unsafe` solo en `crates/vres-win`**, y cada bloque con su comentario `// SAFETY:`. Ninguna función pública de ese crate es `unsafe`.
4. **Los tipos que cruzan la frontera con la interfaz viven en `vres-core`** con `#[derive(TS)]`. `src/lib/ipc.ts` es el único archivo que llama a `invoke`; `src/lib/events.ts`, el único que llama a `listen`. No editar `src/lib/bindings` a mano (se genera con `cargo test`).
5. **Sin colores literales** fuera de `src/styles/` y `src/components/ui/` (`npm run check:tokens`). **Todo texto visible en los tres idiomas** (`src/locales/{es,en,pt-BR}`), con el español neutro como fuente (`npm run check:i18n`).
6. **El motor emite códigos de motivo (`ReasonCode`), nunca frases.** La interfaz traduce.
7. **Prohibido:** hooks de teclado o mouse de bajo nivel (`WH_KEYBOARD_LL`, `WH_MOUSE_LL`), abrir el proceso de un juego (`OpenProcess` solo sobre el propio VRes), inyección, drivers, servicios, conexiones de red nuevas, telemetría y `set_device_event_filter` fuera de `src-tauri/src/lib.rs`. El sondeo de teclas solo existe para atajos de "mantener" armados.
8. **Tauri:** todos los comandos son `async`; las ventanas se crean desde `setup` o desde tareas async (crearlas desde un handler síncrono provoca un deadlock en Windows); la limpieza va en `RunEvent::Exit` y en el hook `on_before_exit` del updater.
9. **Si el hardware contradice el plan,** anotarlo en `docs/hardware/RESULTADOS.md` y consultar antes de improvisar.
10. **No tocar** `%USERPROFILE%\.tauri\` (clave de firma del updater) ni los secretos del repositorio.

## Comandos

```powershell
powershell -ExecutionPolicy Bypass -File scripts\setup-dev.ps1   # una vez: compila fuera de OneDrive
npm ci
npm run build                      # la app de Tauri embebe dist/: hace falta antes de compilar Rust
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace             # también regenera src/lib/bindings
npm run lint
npx vitest --run
npm run check:i18n
npm run check:tokens
npm run check:version
npm run tauri build -- --debug --no-bundle
cargo run -p vres-cli -- --help
```

## Cada fase termina verde

Compila, pasan todas las verificaciones de arriba, se actualizan `CHANGELOG.md`, `docs/ROADMAP.md` y el README si corresponde, se hace commit, se crea el tag `vX.Y.Z` (misma versión en `Cargo.toml` y `package.json`) y el workflow `Release` publica el instalador. No se empieza una fase con la anterior a medias.

## Estilo

- Rust edición 2024, `cargo fmt`, clippy sin advertencias. Comentarios y documentación en español; mensajes de error técnicos (logs) en inglés.
- TypeScript estricto, componentes de función, Tailwind con las clases de los tokens (`bg-surface`, `text-muted`…).
- Commits en español, en modo imperativo ("Agrega…", "Corrige…").
