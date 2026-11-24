import { events } from "@/lib/events";
import { inTauri, ipc } from "@/lib/ipc";
import { useAppStore } from "@/state/app";
import { useEngineStore } from "@/state/engine";
import { useResolutionStore } from "@/state/resolution";

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
  const resolution = useResolutionStore.getState();
  void events.onResolutionChanged((state) => resolution.setState(state));
  void events.onResolutionResolved((resolved) => resolution.setResolved(resolved));
  void events.onSettingsChanged((settings) => resolution.setSettings(settings));
}

/** Modos de pantalla y ajustes: no frenan el arranque si fallan. */
async function loadResolution(): Promise<void> {
  const store = useResolutionStore.getState();
  const [state, settings] = await Promise.allSettled([
    ipc.resolutionGetState(),
    ipc.settingsGet(),
  ]);
  if (state.status === "fulfilled") store.setState(state.value);
  else console.error("resolution_get_state", state.reason);
  if (settings.status === "fulfilled") store.setSettings(settings.value);
  else console.error("settings_get", settings.reason);
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
    await loadResolution();
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
