import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/badge";
import type { DisplayInfo } from "@/lib/bindings/DisplayInfo";
import type { ReasonCode } from "@/lib/bindings/ReasonCode";
import { cn } from "@/lib/utils";

const VENDORS = { nvidia: "NVIDIA", amd: "AMD", intel: "Intel", other: "" } as const;

interface MonitorCardProps {
  index: number;
  display: DisplayInfo;
  checked: boolean;
  /** Limitación de la rampa en este monitor (por ejemplo, HDR activo). */
  reason: ReasonCode | null;
  onToggle: () => void;
}

/** Tarjeta de un monitor, para elegir si el look se le aplica. */
export function MonitorCard({ index, display, checked, reason, onToggle }: MonitorCardProps) {
  const { t } = useTranslation();
  const name = display.friendlyName || display.gdiName;
  const hz = Math.round(display.refreshHz);
  const vendor = VENDORS[display.adapter.vendor];

  return (
    <label
      className={cn(
        "flex cursor-pointer items-start gap-3 rounded-xl border p-4 transition-colors",
        checked ? "border-accent/60 bg-surface-2" : "border-border bg-surface hover:bg-surface-2/60",
      )}
    >
      <span
        aria-hidden
        className="flex size-10 shrink-0 items-center justify-center rounded-lg border border-border text-lg font-semibold"
      >
        {index}
      </span>
      <span className="flex min-w-0 flex-1 flex-col gap-1">
        <span className="truncate font-medium text-fg">{name}</span>
        <span className="text-xs text-muted">
          {display.bounds.width} × {display.bounds.height} · {hz} Hz
          {vendor ? ` · ${vendor}` : ""}
        </span>
        <span className="flex flex-wrap gap-1">
          {display.primary && <Badge>{t("display.targets.primary")}</Badge>}
          {display.internal && <Badge>{t("display.targets.internal")}</Badge>}
          {display.hdr.enabled && <Badge tone="warning">{t("display.targets.hdr")}</Badge>}
        </span>
        {reason && <span className="text-xs text-warning">{t(`reasons.${reason}`)}</span>}
      </span>
      <input
        type="checkbox"
        aria-label={name}
        checked={checked}
        onChange={onToggle}
        className="mt-1 size-4 cursor-pointer accent-accent"
      />
    </label>
  );
}
