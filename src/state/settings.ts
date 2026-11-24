import { create } from "zustand";

import type { ErrorCode } from "@/lib/bindings/ErrorCode";
import type { Settings } from "@/lib/bindings/Settings";
import { ipc, isIpcError } from "@/lib/ipc";

interface SettingsStore {
  /** `settings.json`: atajos, resolución y crosshair. */
  settings: Settings | null;
  setSettings: (settings: Settings) => void;
}

export const useSettingsStore = create<SettingsStore>()((set) => ({
  settings: null,
  setSettings: (settings) => set({ settings }),
}));

/** Guarda los ajustes. Devuelve el código de error, o `null` si salió bien. */
export async function saveSettings(settings: Settings): Promise<ErrorCode | null> {
  try {
    useSettingsStore.getState().setSettings(await ipc.settingsUpdate(settings));
    return null;
  } catch (error) {
    console.error("settings_update", error);
    return isIpcError(error) ? error.code : "internal";
  }
}
