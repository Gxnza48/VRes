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
| Look | `look_preview` · `look_commit` · `look_reset` · `look_set_enabled` · `look_compare` | F1 |
| Motor | `engine_status` · `engine_set_paused` · `engine_emergency_reset` | F1 |
| Pantallas | `displays_list` · `displays_set_targets` · `displays_identify` | F1 |
| Pantallas | `displays_set_trim` | F8 |
| Resolución | `resolution_get_state` · `resolution_apply` (devuelve `{ token, expiresInMs }`) · `resolution_confirm` · `resolution_revert` · `resolution_toggle` · `hdr_set` | F2 |
| Biblioteca | `library_list` · `library_get` · `library_save` · `library_delete` · `library_duplicate` · `library_apply` | F2 |
| Biblioteca | `library_set_favorite` | F5 |
| Compartir | `share_export` · `share_decode` · `share_import` | F5 |
| Crosshair | `crosshair_preview` · `crosshair_commit` · `crosshair_set_enabled` · `crosshair_set_target` · `crosshair_render` (RGBA binario) · `crosshair_import_image` | F4 |
| Juegos y perfiles | `games_list` · `games_add_custom` · `games_remove_custom` · `games_running_candidates` · `profiles_activate` · `profiles_set_auto_switch` | F6 |
| Dark Boost y zoom | `darkboost_preview` · `darkboost_commit` · `zoom_commit` · `enhancers_set_armed` | F7 |
| Monitor | `ddc_get` · `ddc_set` · `ddc_restore_initial` | F8 |
| Ajustes | `settings_get` · `settings_update` · `hotkeys_check` | F1 |
| Ajustes | `customize_set_background` | F8 |
| Diagnóstico | `diag_report` · `diag_export` · `diag_selftest` | F1 |
| Celular | `remote_start` · `remote_stop` · `remote_status` · `remote_regenerate_token` | F9 |
| Popup | `quick_toggle` | F5 |

## Eventos

| Evento | Cuándo | Fase |
|---|---|---|
| `update:progress` | Progreso de descarga de una actualización | F0 ✅ |
| `state:snapshot` | Tras cada cambio confirmado (commit, atajo, cambio automático, celular) | F1 |
| `engine:status` | Cambia el estado de alguna capa | F1 |
| `displays:changed` | Cambio de monitores, modo o HDR (tras el asentamiento) | F1 |
| `app:notice` | Recuperación tras un cierre inesperado, atajo ocupado, archivo corrupto | F1 |
| `foreground:changed` | Cambio de ventana en primer plano | F6 |
| `resolution:pending` · `resolution:resolved` | Cuenta regresiva y su resultado (`kept`, `reverted`, `failed`) | F2 |
| `profile:activated` · `library:changed` | Cambio de perfil activo · cambios en la biblioteca | F5/F6 |
| `remote:clients` | Teléfonos conectados | F9 |

## Desde otros procesos

La ventana de control `VRes.Engine` recibe el mensaje registrado `VRes.Remote.v1` con estos pedidos:

| `wParam` | Pedido | Quién lo usa |
|---|---|---|
| 1 | Restablecer la pantalla (la app sigue abierta) | `vres.exe --reset`, `vres-cli reset`, acceso directo |
| 2 | Cerrar restaurando todo | `vres.exe --shutdown`, `vres-cli shutdown`, instalador |
| 3 | Ping | `vres-cli status` |
