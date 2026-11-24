import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/badge";

export type OverallStatus = "ok" | "limited" | "failed" | "off" | "paused" | "disabled";

const TONES = {
  ok: "success",
  limited: "warning",
  failed: "danger",
  off: "neutral",
  paused: "warning",
  disabled: "neutral",
} as const;

/** Indicador del estado general del motor ("En vivo", "Limitado"…). */
export function StatusBadge({ status }: { status: OverallStatus }) {
  const { t } = useTranslation();
  return (
    <Badge tone={TONES[status]} role="status">
      <span aria-hidden className="size-2 rounded-full bg-current" />
      {t(`display.status.${status}`)}
    </Badge>
  );
}
