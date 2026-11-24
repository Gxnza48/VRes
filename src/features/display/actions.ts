/**
 * Acciones de la pestaña Pantalla.
 *
 * Mientras se arrastra un control, la interfaz manda vistas previas (como
 * mucho una por cuadro); al soltarlo, confirma y el motor lo guarda.
 */
import type { Look } from "@/lib/bindings/Look";
import { ipc } from "@/lib/ipc";
import { useEngineStore } from "@/state/engine";

let pendingPreview: Look | null = null;
let frame: number | null = null;

function report(error: unknown): void {
  console.error("motor", error);
}

/** Actualiza el look en pantalla al instante (vista previa, sin guardar). */
export function previewLook(look: Look): void {
  useEngineStore.getState().setLook(look);
  pendingPreview = look;
  if (frame !== null) return;
  frame = requestAnimationFrame(() => {
    frame = null;
    const next = pendingPreview;
    pendingPreview = null;
    if (next) ipc.lookPreview(next).catch(report);
  });
}

/** Aplica y guarda el look (al soltar un control). */
export function commitLook(look: Look): void {
  if (frame !== null) {
    cancelAnimationFrame(frame);
    frame = null;
  }
  pendingPreview = null;
  useEngineStore.getState().setLook(look);
  ipc
    .lookCommit(look)
    .then((requested) => useEngineStore.getState().setRequested(requested))
    .catch(report);
}

/** Vuelve el look a neutro. */
export function resetLook(): void {
  ipc
    .lookReset()
    .then((requested) => useEngineStore.getState().setRequested(requested))
    .catch(report);
}

/** Enciende o apaga el color. */
export function setColorEnabled(enabled: boolean): void {
  ipc
    .lookSetEnabled(enabled)
    .then((requested) => useEngineStore.getState().setRequested(requested))
    .catch(report);
}

/** Elige a qué pantallas se aplica el look (`null` = todas). */
export function setTargets(targets: string[] | null): void {
  ipc
    .displaysSetTargets(targets)
    .then((requested) => useEngineStore.getState().setRequested(requested))
    .catch(report);
}

/** Comparar con la imagen original mientras se mantiene presionado. */
export function setCompare(active: boolean): void {
  ipc.lookCompare(active).catch(report);
}

/** Muestra el número de cada pantalla. */
export function identifyDisplays(): void {
  ipc.displaysIdentify().catch(report);
}
