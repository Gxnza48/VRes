import { inTauri, ipc } from "@/lib/ipc";
import { useAppStore } from "@/state/app";

/**
 * Arranque de la interfaz: pide los datos iniciales y, cuando ya pintó, le
 * avisa al motor para que muestre la ventana (así no hay destello blanco).
 */
export async function startApp(): Promise<void> {
  if (!inTauri()) return;
  try {
    useAppStore.getState().setBootstrap(await ipc.appBootstrap());
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
