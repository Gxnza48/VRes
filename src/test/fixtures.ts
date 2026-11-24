/** Datos de prueba compartidos por los tests de la interfaz. */
import type { Crosshair } from "@/lib/bindings/Crosshair";
import type { EnhancerSettings } from "@/lib/bindings/EnhancerSettings";
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

/** Dark Boost y Zoom por defecto: desarmados (igual que en Rust). */
export function defaultEnhancers(): EnhancerSettings {
  return {
    darkBoost: {
      armed: false,
      mode: "hold",
      binding: { kind: "mouse", button: "forward" },
      scope: "games",
      effect: {
        strength: 50,
        blackLevel: 20,
        tintEnabled: false,
        tint: "#7CFF6B",
        tintAmount: 40,
        invert: false,
      },
    },
    zoom: {
      armed: false,
      mode: "hold",
      binding: { kind: "mouse", button: "back" },
      scope: "games",
      level: 2,
      displayId: null,
    },
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
    library: { looks: [] },
    profiles: { autoSwitch: false, profiles: [], customGames: [] },
    enhancers: defaultEnhancers(),
    appearance: { theme: "black", background: null, backgroundOpacity: 25 },
    behavior: { closeToTray: true },
  };
}
