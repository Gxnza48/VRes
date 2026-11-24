/**
 * Eventos que emite el motor. Es el ÚNICO archivo que llama a `listen`.
 */
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { DisplayInfo } from "./bindings/DisplayInfo";
import type { EngineStatus } from "./bindings/EngineStatus";
import type { Notice } from "./bindings/Notice";
import type { Requested } from "./bindings/Requested";
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
  /** Progreso de descarga de una actualización. */
  onUpdateProgress: (handler: (progress: UpdateProgress) => void) =>
    on("update:progress", handler),
};
