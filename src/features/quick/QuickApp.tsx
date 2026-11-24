import { useEffect } from "react";
import { useTranslation } from "react-i18next";

import { inTauri, ipc } from "@/lib/ipc";

/**
 * Popup rápido de color (ventana chica siempre encima). Llega en la fase F5;
 * por ahora solo existe la ventana para dejar armado el flujo de compilación.
 */
export function QuickApp() {
  const { t } = useTranslation();

  useEffect(() => {
    if (inTauri()) {
      ipc.appUiReady().catch((error: unknown) => console.error("app_ui_ready", error));
    }
  }, []);

  return (
    <div className="flex h-full flex-col items-center justify-center gap-2 rounded-card border border-border bg-surface p-6 text-center">
      <h1 className="text-lg font-semibold">{t("quick.title")}</h1>
      <p className="text-sm text-muted">{t("quick.description")}</p>
    </div>
  );
}
