/**
 * Puente con el motor de VRes (Rust).
 *
 * Es el ÚNICO archivo que llama a `invoke`: el resto de la interfaz usa `ipc`.
 * Los tipos salen de `src/lib/bindings`, generados desde Rust con ts-rs.
 */
import { invoke, isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";

import type { Bootstrap } from "./bindings/Bootstrap";
import type { IpcError } from "./bindings/IpcError";
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
  /** Datos iniciales: versión, idioma del sistema y modo de arranque. */
  appBootstrap: () => call<Bootstrap>("app_bootstrap"),
  /** Avisa que la interfaz terminó de pintar, para mostrar la ventana. */
  appUiReady: () => call<void>("app_ui_ready"),
  /** Cierra VRes por completo (no solo la ventana). */
  appQuit: () => call<void>("app_quit"),
  /** Consulta si hay una versión nueva en GitHub Releases. */
  updateCheck: () => call<UpdateInfo | null>("update_check"),
  /** Descarga e instala la versión nueva (VRes se reinicia). */
  updateInstall: () => call<void>("update_install"),
  /** Abre un enlace en el navegador del sistema. */
  openExternal: (url: string) => openUrl(url),
};
