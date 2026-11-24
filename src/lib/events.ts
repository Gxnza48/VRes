/**
 * Eventos que emite el motor. Es el ÚNICO archivo que llama a `listen`.
 */
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import type { DisplayInfo } from "./bindings/DisplayInfo";
import type { EngineStatus } from "./bindings/EngineStatus";
import type { ModeResolved } from "./bindings/ModeResolved";
import type { Notice } from "./bindings/Notice";
import type { Requested } from "./bindings/Requested";
import type { ResolutionState } from "./bindings/ResolutionState";
import type { Settings } from "./bindings/Settings";
import type { UpdateProgress } from "./bindings/UpdateProgress";

function on<T>(event: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(event, (message) => handler(message.payload));
}

export const events = {
  /** Cambió el estado de alguna capa. */
  onEngineStatus: (handler: (status: EngineStatus) => void) => on("engine:status", handler),
  /** Cambiaron las pantallas. */
  onDisplaysChanged: (handler: (displays: DisplayInfo[]) => void) =>
    on("displays:changed", handler),
  /** Lo pedido cambió desde fuera de la interfaz (bandeja, atajo de emergencia). */
  onRequestedChanged: (handler: (requested: Requested) => void) =>
    on("state:requested", handler),
  /** Aviso para el usuario. */
  onNotice: (handler: (notice: Notice) => void) => on("app:notice", handler),
  /** Cambiaron los modos de pantalla, o hay un cambio pendiente de confirmar. */
  onResolutionChanged: (handler: (state: ResolutionState) => void) =>
    on("resolution:changed", handler),
  /** Terminó un cambio de resolución (conservado, revertido, vencido o fallido). */
  onResolutionResolved: (handler: (resolved: ModeResolved) => void) =>
    on("resolution:resolved", handler),
  /** Cambiaron los ajustes (por ejemplo, se confirmó un modo nuevo). */
  onSettingsChanged: (handler: (settings: Settings) => void) => on("settings:changed", handler),
  /** Cambió el perfil de juego en uso (`null`: el escritorio). */
  onProfileActive: (handler: (game: string | null) => void) => on("profile:active", handler),
  /** La ventana actual ganó o perdió el foco (el popup rápido se cierra al perderlo). */
  onFocusChanged: (handler: (focused: boolean) => void) =>
    getCurrentWindow().onFocusChanged((event) => handler(event.payload)),
  /** Progreso de descarga de una actualización. */
  onUpdateProgress: (handler: (progress: UpdateProgress) => void) =>
    on("update:progress", handler),
};
