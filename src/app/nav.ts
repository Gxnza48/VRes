import {
  Contrast,
  Crosshair,
  Gamepad2,
  Layers,
  type LucideIcon,
  Monitor,
  MoonStar,
  Paintbrush,
  Scaling,
  Settings,
} from "lucide-react";

/** Secciones de la barra lateral, en orden. */
export const TAB_IDS = [
  "display",
  "library",
  "resolution",
  "monitor",
  "customize",
  "crosshair",
  "profiles",
  "enhancers",
  "more",
] as const;

export type TabId = (typeof TAB_IDS)[number];

export interface TabDefinition {
  id: TabId;
  icon: LucideIcon;
  /** Fase de la hoja de ruta en la que la sección queda funcional. */
  phase: string;
}

export const TABS: readonly TabDefinition[] = [
  { id: "display", icon: Contrast, phase: "F1" },
  { id: "library", icon: Layers, phase: "F5" },
  { id: "resolution", icon: Scaling, phase: "F2" },
  { id: "monitor", icon: Monitor, phase: "F8" },
  { id: "customize", icon: Paintbrush, phase: "F8" },
  { id: "crosshair", icon: Crosshair, phase: "F4" },
  { id: "profiles", icon: Gamepad2, phase: "F6" },
  { id: "enhancers", icon: MoonStar, phase: "F7" },
  { id: "more", icon: Settings, phase: "F0" },
];

export function tabById(id: TabId): TabDefinition {
  return TABS.find((tab) => tab.id === id) ?? TABS[0];
}
