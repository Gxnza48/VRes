/** Datos de prueba compartidos por los tests de la interfaz. */
import type { Crosshair } from "@/lib/bindings/Crosshair";
import type { Settings } from "@/lib/bindings/Settings";

/** El crosshair por defecto de VRes (igual que `Crosshair::default()` en Rust). */
export function defaultCrosshair(): Crosshair {
  return {
    kind: "vector",
    color: "#9EFA12",
    opacity: 100,
    outline: { enabled: true, color: "#000000", thickness: 1, opacity: 100 },
    arms: {
      enabled: true,
      length: 6,
      thickness: 2,
      gap: 3,
      top: true,
      right: true,
      bottom: true,
      left: true,
    },
    dot: { enabled: false, size: 2, round: false },
    ring: { enabled: false, radius: 10, thickness: 1 },
    rotation: 0,
    scale: 100,
    offsetX: 0,
    offsetY: 0,
    image: null,
  };
}

/** Ajustes por defecto. */
export function defaultSettings(): Settings {
  return {
    schemaVersion: 1,
    hotkeys: { resolutionToggle: null },
    resolution: { confirmSeconds: 15, tones: true, toggle: null, confirmed: [] },
    crosshair: {
      enabled: false,
      displayId: null,
      hideFromCapture: false,
      current: defaultCrosshair(),
      saved: [],
    },
  };
}
