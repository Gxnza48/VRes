import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { DisplayMode } from "@/lib/bindings/DisplayMode";
import { cn } from "@/lib/utils";

import { aspectLabel, sizeLabel, stretchLabel } from "./format";

const COLLAPSED = 10;
const MIN_WIDTH = 640;
const MIN_HEIGHT = 480;

interface ModesCardProps {
  modes: DisplayMode[];
  current: DisplayMode;
  /** Resolución de referencia para calcular cuánto estira. */
  native: DisplayMode;
  busy: boolean;
  onApply: (width: number, height: number, refreshHz: number) => void;
}

function sameMode(a: DisplayMode, b: DisplayMode): boolean {
  return a.width === b.width && a.height === b.height && a.refreshHz === b.refreshHz;
}

/** Todos los modos del monitor (con filtros) y el constructor personalizado. */
export function ModesCard({ modes, current, native, busy, onApply }: ModesCardProps) {
  const { t, i18n } = useTranslation();
  const language = i18n.language;
  const [aspect, setAspect] = useState<string | null>(null);
  const [rate, setRate] = useState<number | null>(null);
  const [expanded, setExpanded] = useState(false);

  const aspects = useMemo(
    () => [...new Set(modes.map((mode) => aspectLabel(mode.width, mode.height, language)))],
    [modes, language],
  );
  const rates = useMemo(
    () => [...new Set(modes.map((mode) => mode.refreshHz))].sort((a, b) => b - a),
    [modes],
  );
  const filtered = modes.filter(
    (mode) =>
      (aspect === null || aspectLabel(mode.width, mode.height, language) === aspect) &&
      (rate === null || mode.refreshHz === rate),
  );
  const visible = expanded ? filtered : filtered.slice(0, COLLAPSED);

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle>{t("resolution.modes.title")}</CardTitle>
          <CardDescription>{t("resolution.modes.description")}</CardDescription>
        </CardHeader>
        <CardContent className="gap-4">
          <div className="flex flex-wrap items-center gap-2">
            <span className="mr-1 text-sm text-muted">{t("resolution.modes.aspectFilter")}</span>
            {[null, ...aspects].map((item) => (
              <button
                key={item ?? "all"}
                type="button"
                aria-pressed={aspect === item}
                onClick={() => setAspect(item)}
                className={cn(
                  "rounded-lg border px-2.5 py-1 text-xs transition-colors",
                  aspect === item
                    ? "border-accent bg-accent font-semibold text-accent-fg"
                    : "border-border bg-surface-2 text-muted hover:text-fg",
                )}
              >
                {item ?? t("resolution.modes.all")}
              </button>
            ))}
            <label className="ml-auto flex items-center gap-2 text-sm text-muted">
              {t("resolution.modes.rateFilter")}
              <select
                value={rate ?? ""}
                onChange={(event) =>
                  setRate(event.target.value === "" ? null : Number(event.target.value))
                }
                className="rounded-lg border border-border bg-surface-2 px-2 py-1 text-fg"
              >
                <option value="">{t("resolution.modes.allRates")}</option>
                {rates.map((value) => (
                  <option key={value} value={value}>
                    {value} Hz
                  </option>
                ))}
              </select>
            </label>
          </div>

          {filtered.length === 0 ? (
            <p className="text-sm text-muted">{t("resolution.modes.empty")}</p>
          ) : (
            <ul className="flex flex-col divide-y divide-border rounded-xl border border-border">
              {visible.map((mode) => {
                const isCurrent = sameMode(mode, current);
                return (
                  <li
                    key={`${mode.width}x${mode.height}@${mode.refreshHz}`}
                    className="flex items-center gap-3 px-4 py-2 text-sm"
                  >
                    <span className="w-32 font-medium text-fg tabular-nums">
                      {sizeLabel(mode.width, mode.height)}
                    </span>
                    <span className="w-16 text-muted">
                      {aspectLabel(mode.width, mode.height, language)}
                    </span>
                    <span className="w-16 text-muted tabular-nums">{mode.refreshHz} Hz</span>
                    <span className="flex-1" />
                    {isCurrent ? (
                      <span className="text-xs font-semibold text-accent">
                        {t("resolution.modes.current")}
                      </span>
                    ) : (
                      <Button
                        size="sm"
                        variant="ghost"
                        disabled={busy}
                        onClick={() => onApply(mode.width, mode.height, mode.refreshHz)}
                      >
                        {t("resolution.apply")}
                      </Button>
                    )}
                  </li>
                );
              })}
            </ul>
          )}
          {filtered.length > COLLAPSED && (
            <Button
              variant="ghost"
              size="sm"
              className="self-start"
              onClick={() => setExpanded((value) => !value)}
            >
              {expanded
                ? t("resolution.modes.showLess")
                : t("resolution.modes.showAll", { count: filtered.length })}
            </Button>
          )}
        </CardContent>
      </Card>

      <CustomModeCard modes={modes} native={native} busy={busy} onApply={onApply} />
    </>
  );
}

interface CustomModeCardProps {
  modes: DisplayMode[];
  native: DisplayMode;
  busy: boolean;
  onApply: (width: number, height: number, refreshHz: number) => void;
}

/** Ancho × alto @ Hz a mano, con su proporción y cuánto estira. */
function CustomModeCard({ modes, native, busy, onApply }: CustomModeCardProps) {
  const { t, i18n } = useTranslation();
  const language = i18n.language;
  const [width, setWidth] = useState(1440);
  const [height, setHeight] = useState(native.height);
  const [chosenRate, setChosenRate] = useState<number | null>(null);

  const listedRates = modes
    .filter((mode) => mode.width === width && mode.height === height)
    .map((mode) => mode.refreshHz);
  const rate =
    chosenRate !== null && listedRates.includes(chosenRate) ? chosenRate : (listedRates[0] ?? null);
  const valid = width >= MIN_WIDTH && height >= MIN_HEIGHT;
  const tooLarge = width > native.width || height > native.height;
  const stretch = native.width / native.height / (width / Math.max(height, 1));

  let problem: string | null = null;
  if (!valid) problem = t("resolution.custom.tooSmall");
  else if (tooLarge)
    problem = t("resolution.custom.tooLarge", { size: sizeLabel(native.width, native.height) });
  else if (rate === null) problem = t("resolution.custom.notListed");

  const field = "w-24 rounded-lg border border-border bg-surface-2 px-2 py-1.5 text-fg tabular-nums";

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("resolution.custom.title")}</CardTitle>
        <CardDescription>{t("resolution.custom.description")}</CardDescription>
      </CardHeader>
      <CardContent className="gap-3">
        <div className="flex flex-wrap items-end gap-3 text-sm">
          <label className="flex flex-col gap-1 text-muted">
            {t("resolution.custom.width")}
            <input
              type="number"
              min={MIN_WIDTH}
              max={native.width}
              value={width}
              onChange={(event) => setWidth(Number(event.target.value) || 0)}
              className={field}
            />
          </label>
          <span className="pb-2 text-muted">×</span>
          <label className="flex flex-col gap-1 text-muted">
            {t("resolution.custom.height")}
            <input
              type="number"
              min={MIN_HEIGHT}
              max={native.height}
              value={height}
              onChange={(event) => setHeight(Number(event.target.value) || 0)}
              className={field}
            />
          </label>
          <label className="flex flex-col gap-1 text-muted">
            {t("resolution.custom.rate")}
            <select
              value={rate ?? ""}
              disabled={rate === null}
              onChange={(event) => setChosenRate(Number(event.target.value))}
              className="rounded-lg border border-border bg-surface-2 px-2 py-1.5 text-fg"
            >
              {listedRates.length === 0 && <option value="">—</option>}
              {listedRates.map((value) => (
                <option key={value} value={value}>
                  {value} Hz
                </option>
              ))}
            </select>
          </label>
          <Button
            variant="primary"
            disabled={busy || problem !== null || rate === null}
            onClick={() => rate !== null && onApply(width, height, rate)}
          >
            {t("resolution.apply")}
          </Button>
        </div>
        {valid && (
          <p className="text-sm text-muted">
            {t("resolution.custom.summary", {
              aspect: aspectLabel(width, height, language),
              factor: stretchLabel(stretch, language),
            })}
          </p>
        )}
        {problem && <p className="text-sm text-warning">{problem}</p>}
      </CardContent>
    </Card>
  );
}
