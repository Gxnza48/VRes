/**
 * Acciones de la pestaña Resolución. Cada una marca la pestaña como ocupada
 * mientras el motor trabaja y deja el error traducible en el estado.
 */
import type { ErrorCode } from "@/lib/bindings/ErrorCode";
import type { ModeRequest } from "@/lib/bindings/ModeRequest";
import type { Settings } from "@/lib/bindings/Settings";
import { ipc, isIpcError } from "@/lib/ipc";
import { useResolutionStore } from "@/state/resolution";

function errorCode(error: unknown): ErrorCode {
  return isIpcError(error) ? error.code : "internal";
}

async function run(work: () => Promise<unknown>): Promise<boolean> {
  const store = useResolutionStore.getState();
  store.setError(null);
  store.setResolved(null);
  store.setBusy(true);
  try {
    await work();
    return true;
  } catch (error) {
    console.error("resolution", error);
    store.setError(errorCode(error));
    return false;
  } finally {
    useResolutionStore.getState().setBusy(false);
  }
}

/** Prueba un modo: queda pendiente de confirmación. */
export function applyMode(displayId: string, request: ModeRequest): Promise<boolean> {
  return run(() => ipc.resolutionApply(displayId, request));
}

/** Conserva el modo pendiente. */
export function keepMode(token: string): Promise<boolean> {
  return run(() => ipc.resolutionConfirm(token));
}

/** Vuelve atrás el modo pendiente. */
export function revertMode(token: string): Promise<boolean> {
  return run(() => ipc.resolutionRevert(token));
}

/** Vuelve a la resolución de Windows en una pantalla. */
export function restoreDisplay(displayId: string): Promise<boolean> {
  return run(() => ipc.resolutionRestore(displayId));
}

/** Prende o apaga HDR. */
export function setHdr(displayId: string, enabled: boolean): Promise<boolean> {
  return run(() => ipc.hdrSet(displayId, enabled));
}

/** Guarda los ajustes (atajo, tonos, segundos). */
export async function saveSettings(settings: Settings): Promise<boolean> {
  const store = useResolutionStore.getState();
  store.setError(null);
  try {
    store.setSettings(await ipc.settingsUpdate(settings));
    return true;
  } catch (error) {
    console.error("settings_update", error);
    store.setError(errorCode(error));
    return false;
  }
}
