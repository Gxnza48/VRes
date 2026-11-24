import { Info, X } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { useEngineStore } from "@/state/engine";

/** Avisos del motor (recuperación tras un cierre inesperado, modo seguro…). */
export function Notices() {
  const { t } = useTranslation();
  const notices = useEngineStore((state) => state.notices);
  const dismiss = useEngineStore((state) => state.dismissNotice);

  if (notices.length === 0) return null;
  return (
    <div className="mb-6 flex flex-col gap-2">
      {notices.map((notice) => (
        <div
          key={notice}
          role="alert"
          className="flex items-start gap-3 rounded-xl border border-warning/40 bg-warning/10 px-4 py-3 text-sm text-fg"
        >
          <Info aria-hidden className="mt-0.5 size-4 shrink-0 text-warning" />
          <p className="flex-1">{t(`notices.${notice}`)}</p>
          <Button
            variant="ghost"
            size="sm"
            aria-label={t("notices.dismiss")}
            onClick={() => dismiss(notice)}
            className="-my-1 h-7 px-2"
          >
            <X aria-hidden />
          </Button>
        </div>
      ))}
    </div>
  );
}
