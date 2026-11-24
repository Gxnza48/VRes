import { events } from "@/lib/events";
import { inTauri, ipc } from "@/lib/ipc";
import { useAppStore } from "@/state/app";
import { useEngineStore } from "@/state/engine";

let subscribed = false;

/** Mantiene el estado de la interfaz al día con los eventos del motor. */
function subscribe(): void {
  if (subscribed) return;
  subscribed = true;
  const engine = useEngineStore.getState();
  void events.onEngineStatus((status) => engine.setStatus(status));
  void events.onDisplaysChanged((displays) => engine.setDisplays(displays));
  void events.onRequestedChanged((requested) => engine.setRequested(requested));
  void events.onNotice((notice) => engine.pushNotice(notice));
}

/**
 * Arranque de la interfaz: pide los datos iniciales y, cuando ya pintó, le
 * avisa al motor para que muestre la ventana (así no hay destello blanco).
 */
export async function startApp(): Promise<void> {
  if (!inTauri()) return;
  subscribe();
  try {
    const bootstrap = await ipc.appBootstrap();
    useAppStore.getState().setBootstrap(bootstrap);
    useEngineStore.getState().hydrate(bootstrap);
  } catch (error) {
    console.error("app_bootstrap", error);
  } finally {
    // Dos cuadros: el primero aplica estilos, el segundo ya está en pantalla.
    requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        ipc.appUiReady().catch((error: unknown) => console.error("app_ui_ready", error));
      }),
    );
  }
}
