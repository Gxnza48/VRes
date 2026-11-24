import { create } from "zustand";

import type { Bootstrap } from "@/lib/bindings/Bootstrap";
import type { DisplayInfo } from "@/lib/bindings/DisplayInfo";
import type { EnhancerState } from "@/lib/bindings/EnhancerState";
import type { EngineStatus } from "@/lib/bindings/EngineStatus";
import type { Look } from "@/lib/bindings/Look";
import type { Notice } from "@/lib/bindings/Notice";
import type { Requested } from "@/lib/bindings/Requested";

interface EngineStore {
  /** Llegaron los datos iniciales del motor. */
  ready: boolean;
  displays: DisplayInfo[];
  requested: Requested | null;
  status: EngineStatus | null;
  notices: Notice[];
  /** Perfil de juego en uso (`null`: el escritorio). */
  activeProfile: string | null;
  setActiveProfile: (game: string | null) => void;
  /** Dark Boost y Zoom en este momento. */
  enhancers: EnhancerState;
  setEnhancers: (enhancers: EnhancerState) => void;
  hydrate: (bootstrap: Bootstrap) => void;
  setDisplays: (displays: DisplayInfo[]) => void;
  setRequested: (requested: Requested) => void;
  setLook: (look: Look) => void;
  setStatus: (status: EngineStatus) => void;
  pushNotice: (notice: Notice) => void;
  dismissNotice: (notice: Notice) => void;
}

export const useEngineStore = create<EngineStore>()((set) => ({
  ready: false,
  displays: [],
  requested: null,
  status: null,
  notices: [],
  activeProfile: null,
  setActiveProfile: (activeProfile) => set({ activeProfile }),
  enhancers: { darkBoostListening: false, darkBoost: false, zoomListening: false, zoom: false },
  setEnhancers: (enhancers) => set({ enhancers }),
  hydrate: (bootstrap) =>
    set({
      ready: true,
      displays: bootstrap.displays,
      requested: bootstrap.requested,
      status: bootstrap.status,
      notices: bootstrap.notices,
    }),
  setDisplays: (displays) => set({ displays }),
  setRequested: (requested) => set({ requested }),
  setLook: (look) =>
    set((state) => (state.requested ? { requested: { ...state.requested, look } } : {})),
  setStatus: (status) => set({ status }),
  pushNotice: (notice) =>
    set((state) => ({
      notices: state.notices.includes(notice) ? state.notices : [...state.notices, notice],
    })),
  dismissNotice: (notice) =>
    set((state) => ({ notices: state.notices.filter((item) => item !== notice) })),
}));
