import { create } from "zustand";

import type { CrosshairPreset } from "@/lib/bindings/CrosshairPreset";
import type { GameInfo } from "@/lib/bindings/GameInfo";
import type { LookPreset } from "@/lib/bindings/LookPreset";
import { ipc } from "@/lib/ipc";

interface CatalogStore {
  /** Ya se pidió al motor (no se vuelve a pedir). */
  loaded: boolean;
  lookPresets: LookPreset[];
  crosshairPresets: CrosshairPreset[];
  games: GameInfo[];
  load: () => void;
}

/** Lo que trae VRes de fábrica: looks, crosshairs y juegos. No cambia. */
export const useCatalogStore = create<CatalogStore>()((set, get) => ({
  loaded: false,
  lookPresets: [],
  crosshairPresets: [],
  games: [],
  load: () => {
    if (get().loaded) return;
    set({ loaded: true });
    Promise.all([ipc.looksPresets(), ipc.crosshairPresets(), ipc.gamesList()])
      .then(([lookPresets, crosshairPresets, games]) =>
        set({ lookPresets, crosshairPresets, games }),
      )
      .catch((error: unknown) => {
        console.error("catálogo", error);
        set({ loaded: false });
      });
  },
}));
