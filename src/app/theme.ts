import { useEffect } from "react";

import { useSettingsStore } from "@/state/settings";

/** Aplica el tema elegido en Personalizar (`data-theme` en `<html>`). */
export function useTheme() {
  const theme = useSettingsStore((store) => store.settings?.appearance.theme ?? "black");
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);
}
