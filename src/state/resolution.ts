import { create } from "zustand";

import type { ErrorCode } from "@/lib/bindings/ErrorCode";
import type { ModeResolved } from "@/lib/bindings/ModeResolved";
import type { ResolutionState } from "@/lib/bindings/ResolutionState";
import type { Settings } from "@/lib/bindings/Settings";

interface ResolutionStore {
  /** Modos de cada pantalla y el cambio pendiente (del motor). */
  state: ResolutionState | null;
  /** Ajustes: atajo, tonos, cuenta regresiva y modos confirmados. */
  settings: Settings | null;
  /** Pantalla elegida en la pestaña. */
  selectedId: string | null;
  /** Hay un cambio en curso (aplicar, revertir, HDR). */
  busy: boolean;
  /** Último error, para mostrarlo traducido. */
  error: ErrorCode | null;
  /** Cómo terminó el último cambio. */
  lastResolved: ModeResolved | null;
  setState: (state: ResolutionState) => void;
  setSettings: (settings: Settings) => void;
  select: (displayId: string) => void;
  setBusy: (busy: boolean) => void;
  setError: (error: ErrorCode | null) => void;
  setResolved: (resolved: ModeResolved | null) => void;
}

export const useResolutionStore = create<ResolutionStore>()((set) => ({
  state: null,
  settings: null,
  selectedId: null,
  busy: false,
  error: null,
  lastResolved: null,
  // Los eventos pueden llegar desordenados: se ignora un estado más viejo.
  setState: (state) =>
    set((current) =>
      current.state && Number(state.rev) < Number(current.state.rev) ? {} : { state },
    ),
  setSettings: (settings) => set({ settings }),
  select: (selectedId) => set({ selectedId, error: null }),
  setBusy: (busy) => set({ busy }),
  setError: (error) => set({ error }),
  setResolved: (lastResolved) => set({ lastResolved }),
}));
