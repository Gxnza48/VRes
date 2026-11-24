/**
 * Puente con el motor de VRes (Rust).
 *
 * Es el ÚNICO archivo que llama a `invoke`: el resto de la interfaz usa `ipc`.
 * Los tipos salen de `src/lib/bindings`, generados desde Rust con ts-rs.
 */
import { invoke, isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";

import type { Bootstrap } from "./bindings/Bootstrap";
import type { Crosshair } from "./bindings/Crosshair";
import type { CrosshairPreset } from "./bindings/CrosshairPreset";
import type { DisplayInfo } from "./bindings/DisplayInfo";
import type { EnhancerState } from "./bindings/EnhancerState";
import type { EngineStatus } from "./bindings/EngineStatus";
import type { GameInfo } from "./bindings/GameInfo";
import type { Hotkey } from "./bindings/Hotkey";
import type { HotkeyCheck } from "./bindings/HotkeyCheck";
import type { IpcError } from "./bindings/IpcError";
import type { Look } from "./bindings/Look";
import type { LookPreset } from "./bindings/LookPreset";
import type { ModeRequest } from "./bindings/ModeRequest";
import type { PendingMode } from "./bindings/PendingMode";
import type { Requested } from "./bindings/Requested";
import type { RunningApp } from "./bindings/RunningApp";
import type { ResolutionState } from "./bindings/ResolutionState";
import type { Settings } from "./bindings/Settings";
import type { ShareDecoded } from "./bindings/ShareDecoded";
import type { UpdateInfo } from "./bindings/UpdateInfo";

/** `true` dentro de la app; `false` en un navegador común o en los tests. */
export function inTauri(): boolean {
  return isTauri();
}

/** Distingue los errores del motor (con código estable) de cualquier otro. */
export function isIpcError(value: unknown): value is IpcError {
  return typeof value === "object" && value !== null && "code" in value && "message" in value;
}

function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(command, args);
}

export const ipc = {
  /** Datos iniciales: versión, pantallas, lo pedido, estado y avisos. */
  appBootstrap: () => call<Bootstrap>("app_bootstrap"),
  /** Avisa que la interfaz terminó de pintar, para mostrar la ventana. */
  appUiReady: () => call<void>("app_ui_ready"),
  /** Cierra VRes por completo (no solo la ventana). */
  appQuit: () => call<void>("app_quit"),
  /** Abre la ventana principal (desde el popup rápido). */
  appOpenMain: () => call<void>("app_open_main"),
  /** Abre o cierra el popup rápido. */
  quickToggle: () => call<void>("quick_toggle"),
  /** Cierra el popup rápido. */
  quickClose: () => call<void>("quick_close"),

  /** Pantallas activas. */
  displaysList: () => call<DisplayInfo[]>("displays_list"),
  /** Muestra el número de cada pantalla durante unos segundos. */
  displaysIdentify: () => call<void>("displays_identify"),
  /** Elige a qué pantallas se aplica el look (`null` = todas). */
  displaysSetTargets: (targets: string[] | null) =>
    call<Requested>("displays_set_targets", { targets }),

  /** Vista previa mientras se arrastra un control (no se guarda). */
  lookPreview: (look: Look) => call<void>("look_preview", { look }),
  /** Lo pedido: look, pantallas e interruptores. */
  lookGet: () => call<Requested>("look_get"),
  /** Aplica y guarda el look. */
  lookCommit: (look: Look) => call<Requested>("look_commit", { look }),
  /** Vuelve el look a los valores neutros. */
  lookReset: () => call<Requested>("look_reset"),
  /** Enciende o apaga el color. */
  lookSetEnabled: (enabled: boolean) => call<Requested>("look_set_enabled", { enabled }),
  /** Mientras está activo, se ve la imagen original. */
  lookCompare: (active: boolean) => call<void>("look_compare", { active }),

  /** Pausa o reanuda todo. */
  engineSetPaused: (paused: boolean) => call<Requested>("engine_set_paused", { paused }),
  /** Vuelve la pantalla a la normalidad y apaga el color. */
  engineEmergencyReset: () => call<void>("engine_emergency_reset"),
  /** Estado de cada capa. */
  engineStatus: () => call<EngineStatus>("engine_status"),

  /** `true` si VRes arranca con Windows. */
  settingsGetAutostart: () => call<boolean>("settings_get_autostart"),
  /** Activa o desactiva el arranque con Windows; devuelve el estado final. */
  settingsSetAutostart: (enabled: boolean) => call<boolean>("settings_set_autostart", { enabled }),
  /** Ajustes (atajos y preferencias). */
  settingsGet: () => call<Settings>("settings_get"),
  /** Guarda los ajustes; falla con `hotkeyTaken` si otro programa usa el atajo. */
  settingsUpdate: (settings: Settings) => call<Settings>("settings_update", { settings }),
  /** Revisa un atajo (teclas reservadas y choques conocidos). */
  hotkeysCheck: (hotkey: Hotkey) => call<HotkeyCheck>("hotkeys_check", { hotkey }),
  /** Suspende el atajo de resolución mientras se graba uno nuevo. */
  hotkeysSuspend: (suspended: boolean) => call<void>("hotkeys_suspend", { suspended }),

  /** Modos de cada pantalla, presets y el cambio pendiente. */
  resolutionGetState: () => call<ResolutionState>("resolution_get_state"),
  /** Prueba un modo: si no se confirma a tiempo, vuelve solo. */
  resolutionApply: (displayId: string, request: ModeRequest) =>
    call<PendingMode>("resolution_apply", { displayId, request }),
  /** Conserva el modo pendiente. */
  resolutionConfirm: (token: string) => call<void>("resolution_confirm", { token }),
  /** Vuelve atrás el modo pendiente. */
  resolutionRevert: (token: string) => call<void>("resolution_revert", { token }),
  /** Vuelve a la resolución de Windows en una pantalla. */
  resolutionRestore: (displayId: string) => call<void>("resolution_restore", { displayId }),
  /** Alterna entre la de Windows y la estirada (lo mismo que el atajo). */
  resolutionToggle: () => call<void>("resolution_toggle"),
  /** Prende o apaga HDR (vuelve como estaba al salir de VRes). */
  hdrSet: (displayId: string, enabled: boolean) => call<void>("hdr_set", { displayId, enabled }),

  /** Dibuja un crosshair: lado, centro y RGBA (el mismo dibujo que en pantalla). */
  crosshairRender: (crosshair: Crosshair) => call<ArrayBuffer>("crosshair_render", { crosshair }),
  /** Presets propios de VRes. */
  crosshairPresets: () => call<CrosshairPreset[]>("crosshair_presets"),
  /** Muestra este crosshair en pantalla mientras se edita (`null` vuelve al guardado). */
  crosshairPreview: (crosshair: Crosshair | null) =>
    call<void>("crosshair_preview", { crosshair }),

  /** Código para compartir un look. */
  shareEncodeLook: (look: Look) => call<string>("share_encode_look", { look }),
  /** Código para compartir un crosshair (`null` si es de imagen). */
  shareEncodeCrosshair: (crosshair: Crosshair) =>
    call<string | null>("share_encode_crosshair", { crosshair }),
  /** Código para compartir una resolución. */
  shareEncodeResolution: (mode: ModeRequest) => call<string>("share_encode_resolution", { mode }),
  /** Lee cualquier código de VRes (el motivo de un error viaja en `error`). */
  shareDecode: (code: string) => call<ShareDecoded>("share_decode", { code }),
  /** Looks incluidos en VRes. */
  looksPresets: () => call<LookPreset[]>("looks_presets"),
  /** Juegos conocidos. */
  gamesList: () => call<GameInfo[]>("games_list"),
  /** Perfil de juego en uso (`null`: el escritorio). */
  profileActive: () => call<string | null>("profile_active"),
  /** Dark Boost y Zoom: si escuchan su tecla o están activos. */
  enhancersState: () => call<EnhancerState>("enhancers_state"),
  /** Si una tecla física sirve para Dark Boost o Zoom. */
  enhancersKeySupported: (code: string) => call<boolean>("enhancers_key_supported", { code }),
  /** Programas abiertos con ventana, para agregar un juego propio. */
  gamesRunningCandidates: () => call<RunningApp[]>("games_running_candidates"),

  /** Consulta si hay una versión nueva en GitHub Releases. */
  updateCheck: () => call<UpdateInfo | null>("update_check"),
  /** Descarga e instala la versión nueva (VRes se reinicia). */
  updateInstall: () => call<void>("update_install"),
  /** Abre un enlace en el navegador del sistema. */
  openExternal: (url: string) => openUrl(url),
};
