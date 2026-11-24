import { Eye, MonitorSmartphone, RotateCcw, ShieldCheck } from "lucide-react";
import { useTranslation } from "react-i18next";

import { PageHeader } from "@/components/common/PageHeader";
import { ParamSlider, type SliderRange } from "@/components/common/ParamSlider";
import { type OverallStatus, StatusBadge } from "@/components/common/StatusBadge";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { EngineStatus } from "@/lib/bindings/EngineStatus";
import type { Look } from "@/lib/bindings/Look";
import ranges from "@/lib/bindings/ranges.json";
import type { Requested } from "@/lib/bindings/Requested";
import type { Tone } from "@/lib/bindings/Tone";
import { useEngineStore } from "@/state/engine";

import {
  commitLook,
  identifyDisplays,
  previewLook,
  resetLook,
  setColorEnabled,
  setCompare,
  setTargets,
} from "./actions";
import { AdvancedColor } from "./AdvancedColor";
import { formatDecimal, formatPercent, formatSigned } from "./format";
import { LooksCard } from "./LooksCard";
import { MonitorCard } from "./MonitorCard";

type ToneKey = Exclude<keyof Tone, "gain">;

/** Parámetros del ajuste fino, en el orden en que se muestran. */
const TONE_PARAMS: Array<{ key: ToneKey; range: SliderRange; kind: "percent" | "decimal" | "signed"; scale?: boolean }> = [
  { key: "brightness", range: ranges.brightness, kind: "percent" },
  { key: "contrast", range: ranges.contrast, kind: "percent" },
  { key: "gamma", range: ranges.gamma, kind: "decimal" },
  { key: "temperature", range: ranges.temperature, kind: "signed", scale: true },
  { key: "highlights", range: ranges.tonal, kind: "signed" },
  { key: "shadows", range: ranges.tonal, kind: "signed" },
  { key: "blacks", range: ranges.tonal, kind: "signed" },
  { key: "whites", range: ranges.tonal, kind: "signed" },
  { key: "shadowTint", range: ranges.tonal, kind: "signed", scale: true },
  { key: "highlightTint", range: ranges.tonal, kind: "signed", scale: true },
];

const RANK = { off: 0, ok: 1, limited: 2, failed: 3 } as const;

function overallStatus(requested: Requested, status: EngineStatus | null): OverallStatus {
  if (!requested.colorEnabled) return "disabled";
  if (requested.paused) return "paused";
  let worst: keyof typeof RANK = "off";
  for (const layer of status?.layers ?? []) {
    if (RANK[layer.status] > RANK[worst]) worst = layer.status;
  }
  return worst;
}

/** Pestaña Pantalla: monitores, saturación y ajuste fino. */
export function DisplayPage() {
  const { t, i18n } = useTranslation();
  const ready = useEngineStore((state) => state.ready);
  const displays = useEngineStore((state) => state.displays);
  const requested = useEngineStore((state) => state.requested);
  const status = useEngineStore((state) => state.status);

  if (!ready || !requested) {
    return (
      <>
        <PageHeader title={t("display.title")} subtitle={t("display.subtitle")} />
        <Card>
          <CardDescription>{t("display.unavailable")}</CardDescription>
        </Card>
      </>
    );
  }

  const language = i18n.resolvedLanguage ?? "es";
  const look = requested.look;
  const enabled = requested.colorEnabled;
  const isTicked = (id: string) => requested.targets === null || requested.targets.includes(id);
  const tickedCount = displays.filter((display) => isTicked(display.id)).length;
  const matrixLayer = status?.layers.find((layer) => layer.layer === "matrix");
  const rampReason = (id: string) =>
    status?.layers.find((layer) => layer.layer === "ramp" && layer.display === id)?.reason ?? null;

  const toggleDisplay = (id: string) => {
    const current = displays.filter((display) => isTicked(display.id)).map((display) => display.id);
    const next = current.includes(id) ? current.filter((item) => item !== id) : [...current, id];
    setTargets(next.length === displays.length ? null : next);
  };

  const withTone = (key: ToneKey, value: number): Look => ({
    ...look,
    tone: { ...look.tone, [key]: value },
  });

  const formatValue = (kind: "percent" | "decimal" | "signed", value: number) =>
    kind === "percent"
      ? formatPercent(value, language)
      : kind === "decimal"
        ? formatDecimal(value, language)
        : formatSigned(value);

  const scale: [string, string] = [t("display.scale.cool"), t("display.scale.warm")];

  return (
    <>
      <PageHeader
        title={t("display.title")}
        subtitle={t("display.subtitle")}
        aside={<StatusBadge status={overallStatus(requested, status)} />}
      />

      <div className="mb-4 flex flex-wrap items-center gap-2">
        <Button
          role="switch"
          aria-checked={enabled}
          variant={enabled ? "primary" : "secondary"}
          onClick={() => setColorEnabled(!enabled)}
        >
          <ShieldCheck aria-hidden />
          {t("display.enabled")}: {enabled ? t("display.on") : t("display.off")}
        </Button>
        <Button
          title={t("display.compareHint")}
          disabled={!enabled}
          onPointerDown={() => setCompare(true)}
          onPointerUp={() => setCompare(false)}
          onPointerLeave={() => setCompare(false)}
          onKeyDown={(event) => {
            if ((event.key === " " || event.key === "Enter") && !event.repeat) setCompare(true);
          }}
          onKeyUp={(event) => {
            if (event.key === " " || event.key === "Enter") setCompare(false);
          }}
        >
          <Eye aria-hidden />
          {t("display.compare")}
        </Button>
        <Button variant="ghost" title={t("display.resetHint")} onClick={resetLook}>
          <RotateCcw aria-hidden />
          {t("display.reset")}
        </Button>
      </div>

      <div className="flex flex-col gap-4">
        <Card>
          <CardHeader className="flex-row items-start justify-between gap-4">
            <div className="flex flex-col gap-1">
              <CardTitle>{t("display.targets.title")}</CardTitle>
              <CardDescription>{t("display.targets.description")}</CardDescription>
            </div>
            <Button size="sm" onClick={identifyDisplays}>
              <MonitorSmartphone aria-hidden />
              {t("display.targets.identify")}
            </Button>
          </CardHeader>
          <CardContent>
            {displays.length === 0 ? (
              <p className="text-sm text-muted">{t("display.targets.noDisplays")}</p>
            ) : (
              <div className="grid gap-3 md:grid-cols-2">
                {displays.map((display, index) => (
                  <MonitorCard
                    key={display.id}
                    index={index + 1}
                    display={display}
                    checked={isTicked(display.id)}
                    reason={isTicked(display.id) ? rampReason(display.id) : null}
                    onToggle={() => toggleDisplay(display.id)}
                  />
                ))}
              </div>
            )}
            {tickedCount === 0 && displays.length > 0 && (
              <p className="text-sm text-warning">{t("display.targets.none")}</p>
            )}
            {tickedCount > 0 && tickedCount < displays.length && (
              <p className="text-sm text-warning">{t("display.targets.partial")}</p>
            )}
          </CardContent>
        </Card>

        <LooksCard look={look} enabled={enabled} />

        <Card>
          <CardHeader className="flex-row items-baseline justify-between">
            <CardTitle>{t("display.saturation.title")}</CardTitle>
            <span className="text-xs text-muted">{t("display.saturation.hint")}</span>
          </CardHeader>
          <CardContent>
            <p
              aria-hidden
              className="font-display text-4xl font-black tabular-nums tracking-wide text-accent"
            >
              {formatPercent(look.saturation, language)}
            </p>
            <ParamSlider
              id="saturation"
              label={t("display.saturation.title")}
              hideHeader
              value={look.saturation}
              range={ranges.saturation}
              display={formatPercent(look.saturation, language)}
              disabled={!enabled}
              resetTitle={t("display.resetParam")}
              onChange={(value) => previewLook({ ...look, saturation: value })}
              onCommit={(value) => commitLook({ ...look, saturation: value })}
            />
            {matrixLayer?.reason && (
              <p className="text-sm text-warning">{t(`reasons.${matrixLayer.reason}`)}</p>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex-row items-baseline justify-between">
            <CardTitle>{t("display.vibrance.title")}</CardTitle>
            <span className="text-xs text-muted">{t("display.vibrance.hint")}</span>
          </CardHeader>
          <CardContent>
            <p
              aria-hidden
              className="font-display text-4xl font-black tabular-nums tracking-wide text-accent"
            >
              {Math.round(look.vibrance)}
            </p>
            <ParamSlider
              id="vibrance"
              label={t("display.vibrance.title")}
              hideHeader
              value={look.vibrance}
              range={ranges.vibrance}
              display={String(Math.round(look.vibrance))}
              disabled={!enabled}
              resetTitle={t("display.resetParam")}
              onChange={(value) => previewLook({ ...look, vibrance: value })}
              onCommit={(value) => commitLook({ ...look, vibrance: value })}
            />
            <ul className="flex flex-col gap-1.5 text-sm">
              {displays.map((display, index) => {
                const support = status?.vibrance?.[display.id];
                const reason = status?.layers.find(
                  (layer) => layer.layer === "vibrance" && layer.display === display.id,
                )?.reason;
                return (
                  <li key={display.id} className="flex flex-wrap items-center gap-2">
                    <span className="font-display font-black text-muted">{index + 1}</span>
                    <span className="text-fg">{display.friendlyName || display.gdiName}</span>
                    <Badge tone={support ? "accent" : "neutral"}>
                      {support
                        ? t("display.vibrance.driver", {
                            backend: support.backend === "nvidia" ? "NVIDIA" : "AMD",
                          })
                        : t("display.vibrance.emulated")}
                    </Badge>
                    {support && (
                      <span className="text-xs text-muted">
                        {t("display.vibrance.original", { value: support.original })}
                      </span>
                    )}
                    {reason && isTicked(display.id) && (
                      <span className="text-xs text-warning">{t(`reasons.${reason}`)}</span>
                    )}
                  </li>
                );
              })}
            </ul>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{t("display.fineTune.title")}</CardTitle>
            <CardDescription>{t("display.fineTune.description")}</CardDescription>
          </CardHeader>
          <CardContent className="grid gap-x-8 gap-y-5 md:grid-cols-2">
            {TONE_PARAMS.map(({ key, range, kind, scale: hasScale }) => (
              <ParamSlider
                key={key}
                id={`tone-${key}`}
                label={t(`display.params.${key}`)}
                value={look.tone[key]}
                range={range}
                display={formatValue(kind, look.tone[key])}
                scale={hasScale ? scale : undefined}
                disabled={!enabled}
                resetTitle={t("display.resetParam")}
                onChange={(value) => previewLook(withTone(key, value))}
                onCommit={(value) => commitLook(withTone(key, value))}
              />
            ))}
          </CardContent>
        </Card>

        <AdvancedColor look={look} enabled={enabled} />

        <p className="text-xs text-muted">{t("display.emergency")}</p>
      </div>
    </>
  );
}
