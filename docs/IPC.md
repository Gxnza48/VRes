# Contrato interfaz ⇄ motor

La interfaz (React) habla con el motor (Rust) solo a través de `src/lib/ipc.ts` (comandos) y `src/lib/events.ts` (eventos).

**Convenciones:**

- Comandos `snake_case` con prefijo de función, **siempre `async`**.
- Devuelven `Result<T, IpcError>`; `IpcError { code, message }` lleva un código estable (la interfaz traduce por código) y un detalle técnico en inglés para los logs.
- Los tipos viven en `vres-core` con `#[derive(TS)]` y se generan en `src/lib/bindings` con `cargo test`. CI falla si quedaron desactualizados.
- Se envían objetos completos (`Look`, `Settings`), sin tipos "parche".
- Los eventos usan `:` como separador.

## Comandos

| Grupo | Comando | Fase |
|---|---|---|
| App | `app_bootstrap` · `app_ui_ready` · `app_quit` | F0 ✅ |
| Actualizaciones | `update_check` · `update_install` | F0 ✅ |
| App | `app_get_snapshot` | F1 |
| Look | `look_preview` · `look_commit` · `look_reset` · `look_set_enabled` · `look_compare` | F1 ✅ |
| Motor | `engine_status` · `engine_set_paused` · `engine_emergency_reset` | F1 ✅ |
| Pantallas | `displays_list` · `displays_set_targets` · `displays_identify` | F1 ✅ |
| Pantallas | `displays_set_trim` | F8 |
| Resolución | `resolution_get_state` · `resolution_apply` (devuelve `PendingMode`: `token`, `expiresInMs`, `seconds`) · `resolution_confirm` · `resolution_revert` · `resolution_restore` · `resolution_toggle` · `hdr_set` | F2 ✅ |
| Biblioteca | Looks guardados, favoritos, crosshairs guardados y resoluciones conservadas viven en `settings.json` (`settings_get` · `settings_update`); `looks_presets` · `games_list` | F5 ✅ |
| Compartir | `share_encode_look` · `share_encode_crosshair` · `share_encode_resolution` · `share_decode` (nunca falla: el motivo viaja en `error`) | F5 ✅ |
| Crosshair | `crosshair_render` (bytes: lado, centro y RGBA) · `crosshair_presets` · `crosshair_preview`; prender, elegir monitor, guardar e imagen propia van por `settings_update` | F4 ✅ |
| Juegos y perfiles | `games_list` (con ejecutables y anti-cheat) · `games_running_candidates` · `profile_active`; los perfiles, el cambio automático y los juegos propios van por `settings_update` | F6 ✅ |
| Dark Boost y zoom | `enhancers_state` · `enhancers_key_supported`; armar, teclas y efectos van por `settings_update` | F7 ✅ |
| Monitor | `ddc_get` · `ddc_set` · `ddc_restore_initial` | F8 |
| Ajustes | `settings_get_autostart` · `settings_set_autostart` | F1 ✅ |
| Ajustes | `settings_get` · `settings_update` (falla con `hotkeyTaken` si otro programa usa el atajo) · `hotkeys_check` · `hotkeys_suspend` | F2 ✅ |
| Ajustes | `customize_set_background` | F8 |
| Diagnóstico | `diag_report` · `diag_export` · `diag_selftest` | F1 |
| Celular | `remote_start` · `remote_stop` · `remote_status` · `remote_regenerate_token` | F9 |
| Popup | `quick_toggle` · `quick_close` · `app_open_main` · `look_get` | F5 ✅ |

## Eventos

| Evento | Cuándo | Fase |
|---|---|---|
| `update:progress` | Progreso de descarga de una actualización | F0 ✅ |
| `state:snapshot` | Tras cada cambio confirmado (commit, atajo, cambio automático, celular) | F1 |
| `engine:status` | Cambia el estado de alguna capa | F1 ✅ |
| `displays:changed` | Cambio de monitores, modo o HDR (tras el asentamiento) | F1 ✅ |
| `app:notice` | Recuperación tras un cierre inesperado, modo seguro, atajo ocupado, restablecimiento | F1 ✅ |
| `state:requested` | Lo pedido cambió fuera de la interfaz (bandeja, atajo de emergencia) | F1 ✅ |
| `resolution:changed` | Cambiaron los modos o hay un cambio pendiente (incluye `pending`) | F2 ✅ |
| `resolution:resolved` | Terminó un cambio: `kept`, `reverted`, `expired` o `failed` | F2 ✅ |
| `settings:changed` | Cambiaron los ajustes (por ejemplo, se conservó un modo nuevo) | F2 ✅ |
| `profile:active` | Cambió el perfil de juego en uso (`null`: el escritorio) | F6 ✅ |
| `enhancers:state` | Dark Boost o Zoom empezaron o dejaron de escuchar su tecla, o de estar activos | F7 ✅ |
| `remote:clients` | Teléfonos conectados | F9 |

## Desde otros procesos

La ventana de control `VRes.Engine` recibe el mensaje registrado `VRes.Remote.v1` con estos pedidos:

| `wParam` | Pedido | Quién lo usa |
|---|---|---|
| 1 | Restablecer la pantalla (la app sigue abierta) | `vres.exe --reset`, `vres-cli reset`, acceso directo |
| 2 | Cerrar restaurando todo | `vres.exe --shutdown`, `vres-cli shutdown`, instalador |
| 3 | Ping | `vres-cli status` |
