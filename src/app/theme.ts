import { useEffect } from "react";

import { useSettingsStore } from "@/state/settings";

/**
 * Aplica el tema y el color de acento elegidos en Personalizar
 * (`data-theme` y `data-accent` en `<html>`). Con `scale`, también el tamaño
 * de la interfaz (solo en la ventana principal: el popup rápido tiene un
 * tamaño fijo).
 */
export function useTheme({ scale = false }: { scale?: boolean } = {}) {
  const theme = useSettingsStore((store) => store.settings?.appearance.theme ?? "black");
  const accent = useSettingsStore((store) => store.settings?.appearance.accent ?? "lime");
  const uiScale = useSettingsStore((store) => store.settings?.appearance.uiScale ?? 100);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    document.documentElement.dataset.accent = accent;
  }, [theme, accent]);

  useEffect(() => {
    if (!scale) return;
    // Todo usa rem: cambiar el tamaño de letra de la raíz escala la interfaz entera.
    document.documentElement.style.fontSize = uiScale === 100 ? "" : `${uiScale}%`;
  }, [scale, uiScale]);
}
