import { create } from "zustand";

import type { TabId } from "@/app/nav";
import type { Bootstrap } from "@/lib/bindings/Bootstrap";

interface AppState {
  /** Datos iniciales del motor (null hasta que llegan, o fuera de la app). */
  bootstrap: Bootstrap | null;
  /** Sección visible. */
  tab: TabId;
  setBootstrap: (bootstrap: Bootstrap | null) => void;
  setTab: (tab: TabId) => void;
}

export const useAppStore = create<AppState>()((set) => ({
  bootstrap: null,
  tab: "home",
  setBootstrap: (bootstrap) => set({ bootstrap }),
  setTab: (tab) => set({ tab }),
}));
