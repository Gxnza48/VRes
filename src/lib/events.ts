/**
 * Eventos que emite el motor. Es el ÚNICO archivo que llama a `listen`.
 */
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { UpdateProgress } from "./bindings/UpdateProgress";

export const events = {
  /** Progreso de descarga de una actualización. */
  onUpdateProgress: (handler: (progress: UpdateProgress) => void): Promise<UnlistenFn> =>
    listen<UpdateProgress>("update:progress", (event) => handler(event.payload)),
};
