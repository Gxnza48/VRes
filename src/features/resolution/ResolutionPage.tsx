import { RotateCcw } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { PageHeader } from "@/components/common/PageHeader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { ModeResolved } from "@/lib/bindings/ModeResolved";
import type { Scaling } from "@/lib/bindings/Scaling";
import { cn } from "@/lib/utils";
import { useEngineStore } from "@/state/engine";
import { useResolutionStore } from "@/state/resolution";
import { useSettingsStore } from "@/state/settings";

import { applyMode, restoreDisplay, setHdr } from "./actions";
import { aspectLabel, sizeLabel } from "./format";
import { HotkeyCard } from "./HotkeyCard";
import { ModesCard } from "./ModesCard";
import { PresetCard } from "./PresetCard";

/** Cómo terminó el último cambio, en una línea. */
function ResolvedLine({ resolved }: { resolved: ModeResolved }) {
  const { t } = useTranslation();
  const mode = sizeLabel(resolved.mode.width, resolved.mode.height);
  const tone =
    resolved.outcome === "kept"
      ? "text-accent"
      : resolved.outcome === "failed"
        ? "text-danger"
        : "text-muted";
  return (
    <p role="status" className={cn("text-sm", tone)}>
      {t(`resolution.resolved.${resolved.outcome}`, { mode })}
    </p>
  );
}

/** Pestaña Resolución: modo actual, estiradas, todos los modos, HDR y atajo. */
export function ResolutionPage() {
  const { t, i18n } = useTranslation();
  const displays = useEngineStore((state) => state.displays);
  const state = useResolutionStore((store) => store.state);
  const settings = useSettingsStore((store) => store.settings);
  const selectedId = useResolutionStore((store) => store.selectedId);
  const select = useResolutionStore((store) => store.select);
  const busy = useResolutionStore((store) => store.busy);
  const error = useResolutionStore((store) => store.error);
  const lastResolved = useResolutionStore((store) => store.lastResolved);
  const [scaling, setScaling] = useState<Scaling>("stretched");

  const header = (
    <PageHeader title={t("resolution.title")} subtitle={t("resolution.subtitle")} />
  );
  if (!state) {
    return (
      <div className="flex flex-col gap-6">
        {header}
        <Card>
          <CardContent>
            <p className="text-sm text-muted">{t("resolution.loading")}</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  const modes =
    state.displays.find((item) => item.displayId === selectedId) ?? state.displays[0] ?? null;
  if (!modes) {
    return (
      <div className="flex flex-col gap-6">
        {header}
        <p className="text-sm text-muted">{t("display.targets.noDisplays")}</p>
      </div>
    );
  }
  const display = displays.find((item) => item.id === modes.displayId);
  const name = display?.friendlyName || display?.gdiName || modes.displayId;
  const native = modes.native ?? modes.saved;
  const apply = (width: number, height: number, refreshHz: number) =>
    void applyMode(modes.displayId, { width, height, refreshHz, scaling });

  return (
    <div className="flex flex-col gap-6">
      {header}

      {error && (
        <p
          role="alert"
          className="rounded-xl border border-danger/40 bg-danger/10 px-4 py-3 text-sm text-fg"
        >
          {t(`errors.${error}`)}
        </p>
      )}
      {lastResolved && !error && <ResolvedLine resolved={lastResolved} />}

      {state.displays.length > 1 && (
        <div role="tablist" aria-label={t("resolution.picker")} className="flex flex-wrap gap-2">
          {state.displays.map((item, index) => {
            const info = displays.find((candidate) => candidate.id === item.displayId);
            const active = item.displayId === modes.displayId;
            return (
              <button
                key={item.displayId}
                type="button"
                role="tab"
                aria-selected={active}
                onClick={() => select(item.displayId)}
                className={cn(
                  "flex items-center gap-2 rounded-xl border px-3 py-2 text-sm transition-colors",
                  active
                    ? "border-accent bg-accent font-semibold text-accent-fg"
                    : "border-border bg-surface text-muted hover:text-fg",
                )}
              >
                <span className="font-display font-black">{index + 1}</span>
                <span>{info?.friendlyName || info?.gdiName || item.displayId}</span>
                {item.changed && (
                  <span className={cn("size-2 rounded-full", active ? "bg-accent-fg" : "bg-accent")} />
                )}
              </button>
            );
          })}
        </div>
      )}

      <Card>
        <CardHeader className="flex-row items-start justify-between gap-4">
          <div className="flex flex-col gap-1.5">
            <CardTitle>{t("resolution.current.title")}</CardTitle>
            <CardDescription>{name}</CardDescription>
          </div>
          {modes.changed && <Badge tone="accent">{t("resolution.current.temporary")}</Badge>}
        </CardHeader>
        <CardContent className="gap-4">
          <p className="font-display text-4xl font-black tracking-wide text-accent tabular-nums">
            {sizeLabel(modes.current.width, modes.current.height)}
            <span className="ml-3 text-2xl text-muted">{modes.current.refreshHz} Hz</span>
          </p>
          <dl className="grid grid-cols-2 gap-x-6 gap-y-2 text-sm md:grid-cols-4">
            <div>
              <dt className="text-muted">{t("resolution.current.aspect")}</dt>
              <dd className="text-fg">
                {aspectLabel(modes.current.width, modes.current.height, i18n.language)}
              </dd>
            </div>
            <div>
              <dt className="text-muted">{t("resolution.current.scaling")}</dt>
              <dd className="text-fg">{t(`resolution.scaling.${modes.scaling}`)}</dd>
            </div>
            <div>
              <dt className="text-muted">{t("resolution.current.saved")}</dt>
              <dd className="text-fg">
                {sizeLabel(modes.saved.width, modes.saved.height)} · {modes.saved.refreshHz} Hz
              </dd>
            </div>
            <div>
              <dt className="text-muted">{t("resolution.current.native")}</dt>
              <dd className="text-fg">{sizeLabel(native.width, native.height)}</dd>
            </div>
          </dl>
          <div className="flex flex-wrap items-center gap-4">
            {modes.changed && (
              <Button disabled={busy} onClick={() => void restoreDisplay(modes.displayId)}>
                <RotateCcw aria-hidden />
                {t("resolution.current.restore")}
              </Button>
            )}
            {display?.hdr.supported ? (
              <label className="flex items-center gap-2 text-sm text-fg">
                <input
                  type="checkbox"
                  role="switch"
                  checked={display.hdr.enabled}
                  disabled={busy}
                  onChange={(event) => void setHdr(modes.displayId, event.target.checked)}
                  className="size-4 accent-accent"
                />
                {t("resolution.current.hdr")}
              </label>
            ) : (
              <span className="text-xs text-muted">{t("resolution.current.hdrUnsupported")}</span>
            )}
            {busy && <span className="text-sm text-muted">{t("resolution.busy")}</span>}
          </div>
          {display?.hdr.supported && (
            <p className="text-xs text-muted">{t("resolution.current.hdrHint")}</p>
          )}
        </CardContent>
      </Card>

      <PresetCard
        presets={modes.presets}
        scaling={scaling}
        busy={busy}
        onScalingChange={setScaling}
        onApply={apply}
      />

      <ModesCard
        key={modes.displayId}
        modes={modes.modes}
        current={modes.current}
        native={native}
        busy={busy}
        onApply={apply}
      />

      {settings && <HotkeyCard settings={settings} displays={displays} busy={busy} />}

    </div>
  );
}
