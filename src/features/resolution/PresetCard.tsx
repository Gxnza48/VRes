import { useTranslation } from "react-i18next";

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { ResolutionPreset } from "@/lib/bindings/ResolutionPreset";
import type { Scaling } from "@/lib/bindings/Scaling";
import { cn } from "@/lib/utils";

import { sizeLabel, stretchLabel } from "./format";

/** Escalados que el usuario puede elegir. */
const SCALING_CHOICES: readonly Scaling[] = ["stretched", "aspectRatio", "centered"];

interface PresetCardProps {
  presets: ResolutionPreset[];
  scaling: Scaling;
  busy: boolean;
  onScalingChange: (scaling: Scaling) => void;
  onApply: (width: number, height: number, refreshHz: number) => void;
}

/** Resoluciones estiradas propuestas para la pantalla elegida. */
export function PresetCard({ presets, scaling, busy, onScalingChange, onApply }: PresetCardProps) {
  const { t, i18n } = useTranslation();

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("resolution.presets.title")}</CardTitle>
        <CardDescription>{t("resolution.presets.description")}</CardDescription>
      </CardHeader>
      <CardContent className="gap-4">
        <div role="radiogroup" aria-label={t("resolution.scaling.title")} className="flex flex-wrap items-center gap-2">
          <span className="mr-1 text-sm text-muted">{t("resolution.scaling.title")}</span>
          {SCALING_CHOICES.map((choice) => (
            <button
              key={choice}
              type="button"
              role="radio"
              aria-checked={scaling === choice}
              onClick={() => onScalingChange(choice)}
              className={cn(
                "rounded-lg border px-3 py-1.5 text-sm transition-colors",
                scaling === choice
                  ? "border-accent bg-accent font-semibold text-accent-fg"
                  : "border-border bg-surface-2 text-muted hover:text-fg",
              )}
            >
              {t(`resolution.scaling.${choice}`)}
            </button>
          ))}
        </div>

        {presets.length === 0 ? (
          <p className="text-sm text-muted">{t("resolution.presets.none")}</p>
        ) : (
          <div className="grid grid-cols-2 gap-3 lg:grid-cols-3">
            {presets.map((preset) => (
              <button
                key={`${preset.width}x${preset.height}`}
                type="button"
                disabled={!preset.available || busy}
                title={preset.available ? undefined : t("resolution.presets.unavailable")}
                onClick={() => onApply(preset.width, preset.height, preset.refreshHz)}
                className={cn(
                  "flex flex-col items-start gap-1 rounded-xl border p-4 text-left transition-colors",
                  preset.available
                    ? "border-border bg-surface-2 hover:border-accent disabled:opacity-50"
                    : "cursor-not-allowed border-dashed border-border bg-surface opacity-60",
                )}
              >
                <span className="text-xs font-semibold tracking-wide text-accent uppercase">
                  {t(`resolution.presets.groups.${preset.group}`)}
                </span>
                <span className="font-display text-lg font-black text-fg tabular-nums">
                  {sizeLabel(preset.width, preset.height)}
                </span>
                <span className="text-xs text-muted">
                  {preset.available
                    ? `${preset.refreshHz} Hz · ${t("resolution.presets.stretch", {
                        factor: stretchLabel(preset.stretch, i18n.language),
                      })}`
                    : t("resolution.presets.unavailable")}
                </span>
              </button>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
