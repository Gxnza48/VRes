import { MoonStar, ShieldAlert, ZoomIn } from "lucide-react";
import { useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { PageHeader } from "@/components/common/PageHeader";
import { ParamSlider, type SliderRange } from "@/components/common/ParamSlider";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { Binding } from "@/lib/bindings/Binding";
import type { DarkBoost } from "@/lib/bindings/DarkBoost";
import type { EnhancerScope } from "@/lib/bindings/EnhancerScope";
import type { EnhancerSettings } from "@/lib/bindings/EnhancerSettings";
import type { HoldMode } from "@/lib/bindings/HoldMode";
import ranges from "@/lib/bindings/ranges.json";
import { cn } from "@/lib/utils";
import { useEngineStore } from "@/state/engine";
import { saveSettings, useSettingsStore } from "@/state/settings";

import { sameBinding } from "./binding";
import { BindingRecorder } from "./BindingRecorder";

const field = "rounded-lg border border-border bg-surface-2 px-2 py-1.5 text-sm text-fg";

interface CommonControls {
  armed: boolean;
  mode: HoldMode;
  binding: Binding;
  scope: EnhancerScope;
}

/** Pestaña Dark Boost y Zoom. */
export function EnhancersPage() {
  const { t } = useTranslation();
  const settings = useSettingsStore((store) => store.settings);
  const displays = useEngineStore((store) => store.displays);
  const live = useEngineStore((store) => store.enhancers);
  // Valor de un deslizador mientras se arrastra (se guarda al soltar).
  const [drafts, setDrafts] = useState<Record<string, number>>({});

  if (!settings) {
    return <PageHeader title={t("enhancers.title")} subtitle={t("enhancers.subtitle")} />;
  }
  const config = settings.enhancers;
  const save = (next: EnhancerSettings) => void saveSettings({ ...settings, enhancers: next });
  const saveDark = (patch: Partial<EnhancerSettings["darkBoost"]>) =>
    save({ ...config, darkBoost: { ...config.darkBoost, ...patch } });
  const saveEffect = (patch: Partial<DarkBoost>) =>
    saveDark({ effect: { ...config.darkBoost.effect, ...patch } });
  const saveZoom = (patch: Partial<EnhancerSettings["zoom"]>) =>
    save({ ...config, zoom: { ...config.zoom, ...patch } });

  const slider = (
    id: string,
    label: string,
    value: number,
    range: SliderRange,
    display: (value: number) => string,
    commit: (value: number) => void,
  ) => (
    <ParamSlider
      id={`enhancers-${id}`}
      label={label}
      value={drafts[id] ?? value}
      range={range}
      display={display(drafts[id] ?? value)}
      resetTitle={t("display.resetParam")}
      onChange={(next) => setDrafts((current) => ({ ...current, [id]: next }))}
      onCommit={(next) => {
        setDrafts((current) =>
          Object.fromEntries(Object.entries(current).filter(([key]) => key !== id)),
        );
        commit(next);
      }}
    />
  );
  const percent = (value: number) => t("enhancers.units.percent", { value: Math.round(value) });
  const times = (value: number) => t("enhancers.units.times", { value: value.toFixed(1) });

  /** Estado en vivo: desarmado, activo, listo o esperando. */
  const stateBadge = (
    armed: boolean,
    active: boolean,
    listening: boolean,
    scope: EnhancerScope,
  ) => {
    if (!armed) return <Badge>{t("enhancers.state.disarmed")}</Badge>;
    if (active) return <Badge tone="accent">{t("enhancers.state.active")}</Badge>;
    if (listening) return <Badge tone="success">{t("enhancers.state.listening")}</Badge>;
    return (
      <Badge>
        {t(scope === "games" ? "enhancers.state.waitingGame" : "enhancers.state.paused")}
      </Badge>
    );
  };

  /** Armar, modo, tecla y alcance: iguales para los dos. */
  const common = (
    id: string,
    controls: CommonControls,
    update: (patch: Partial<CommonControls>) => void,
  ) => (
    <div className="grid gap-4 md:grid-cols-3">
      <label className="flex flex-col gap-1.5 text-sm text-muted">
        {t("enhancers.mode.label")}
        <select
          value={controls.mode}
          onChange={(event) => update({ mode: event.target.value as HoldMode })}
          className={field}
        >
          <option value="hold">{t("enhancers.mode.hold")}</option>
          <option value="toggle">{t("enhancers.mode.toggle")}</option>
        </select>
      </label>
      <BindingRecorder
        id={`${id}-binding`}
        value={controls.binding}
        onChange={(binding) => update({ binding })}
      />
      <label className="flex flex-col gap-1.5 text-sm text-muted">
        {t("enhancers.scope.label")}
        <select
          value={controls.scope}
          onChange={(event) => update({ scope: event.target.value as EnhancerScope })}
          className={field}
        >
          <option value="games">{t("enhancers.scope.games")}</option>
          <option value="always">{t("enhancers.scope.always")}</option>
        </select>
      </label>
    </div>
  );

  const header = (
    icon: ReactNode,
    title: string,
    description: string,
    badge: ReactNode,
    armed: boolean,
    toggle: () => void,
  ) => (
    <CardHeader className="flex-row items-start justify-between gap-4">
      <div className="flex flex-col gap-1.5">
        <CardTitle className="flex items-center gap-2">
          {icon}
          {title}
        </CardTitle>
        <CardDescription>{description}</CardDescription>
      </div>
      <div className="flex shrink-0 items-center gap-2">
        {badge}
        <Button variant={armed ? "secondary" : "primary"} aria-pressed={armed} onClick={toggle}>
          {armed ? t("enhancers.disarm") : t("enhancers.arm")}
        </Button>
      </div>
    </CardHeader>
  );

  const dark = config.darkBoost;
  const zoom = config.zoom;
  const clash = dark.armed && zoom.armed && sameBinding(dark.binding, zoom.binding);

  return (
    <div className="flex flex-col gap-6">
      <PageHeader title={t("enhancers.title")} subtitle={t("enhancers.subtitle")} />

      <p className="flex items-start gap-3 rounded-xl border border-warning/40 bg-warning/10 px-4 py-3 text-sm text-fg">
        <ShieldAlert aria-hidden className="mt-0.5 size-4 shrink-0 text-warning" />
        {t("enhancers.warning")}
      </p>
      {clash && (
        <p role="alert" className="text-sm text-warning">
          {t("enhancers.binding.same")}
        </p>
      )}

      <Card className={cn(dark.armed && "border-accent/50")}>
        {header(
          <MoonStar aria-hidden className="size-4 text-accent" />,
          t("enhancers.darkBoost.title"),
          t("enhancers.darkBoost.description"),
          stateBadge(dark.armed, live.darkBoost, live.darkBoostListening, dark.scope),
          dark.armed,
          () => saveDark({ armed: !dark.armed }),
        )}
        <CardContent className="flex flex-col gap-5">
          {common("dark", dark, (patch) => saveDark(patch))}
          <div className="grid gap-5 md:grid-cols-2">
            {slider(
              "strength",
              t("enhancers.darkBoost.strength"),
              dark.effect.strength,
              ranges.darkBoost,
              percent,
              (strength) => saveEffect({ strength }),
            )}
            {slider(
              "blackLevel",
              t("enhancers.darkBoost.blackLevel"),
              dark.effect.blackLevel,
              ranges.blackLevel,
              percent,
              (blackLevel) => saveEffect({ blackLevel }),
            )}
          </div>
          <div className="grid items-end gap-5 md:grid-cols-2">
            <div className="flex flex-wrap items-center gap-3 text-sm text-fg">
              <label className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={dark.effect.tintEnabled}
                  onChange={(event) => saveEffect({ tintEnabled: event.target.checked })}
                  className="size-4 accent-accent"
                />
                {t("enhancers.darkBoost.tint")}
              </label>
              {dark.effect.tintEnabled && (
                <input
                  type="color"
                  aria-label={t("enhancers.darkBoost.tintColor")}
                  value={dark.effect.tint}
                  onChange={(event) => saveEffect({ tint: event.target.value.toUpperCase() })}
                  className="h-8 w-12 cursor-pointer rounded border border-border bg-surface-2"
                />
              )}
            </div>
            {dark.effect.tintEnabled &&
              slider(
                "tintAmount",
                t("enhancers.darkBoost.tintAmount"),
                dark.effect.tintAmount,
                ranges.tintAmount,
                percent,
                (tintAmount) => saveEffect({ tintAmount }),
              )}
          </div>
          <div className="flex flex-col gap-2">
            <label className="flex items-center gap-2 text-sm text-fg">
              <input
                type="checkbox"
                checked={dark.effect.invert}
                onChange={(event) => saveEffect({ invert: event.target.checked })}
                className="size-4 accent-accent"
              />
              {t("enhancers.darkBoost.invert")}
            </label>
            {dark.effect.invert && (
              <p role="alert" className="text-xs text-warning">
                {t("enhancers.darkBoost.invertWarning")}
              </p>
            )}
          </div>
        </CardContent>
      </Card>

      <Card className={cn(zoom.armed && "border-accent/50")}>
        {header(
          <ZoomIn aria-hidden className="size-4 text-accent" />,
          t("enhancers.zoom.title"),
          t("enhancers.zoom.description"),
          stateBadge(zoom.armed, live.zoom, live.zoomListening, zoom.scope),
          zoom.armed,
          () => saveZoom({ armed: !zoom.armed }),
        )}
        <CardContent className="flex flex-col gap-5">
          {common("zoom", zoom, (patch) => saveZoom(patch))}
          <div className="grid gap-5 md:grid-cols-2">
            {slider("level", t("enhancers.zoom.level"), zoom.level, ranges.zoom, times, (level) =>
              saveZoom({ level: Math.round(level * 10) / 10 }),
            )}
            <label className="flex flex-col gap-1.5 text-sm text-muted">
              {t("enhancers.zoom.display")}
              <select
                value={zoom.displayId ?? ""}
                onChange={(event) => saveZoom({ displayId: event.target.value || null })}
                className={field}
              >
                <option value="">{t("enhancers.zoom.primary")}</option>
                {displays.map((display) => (
                  <option key={display.id} value={display.id}>
                    {display.friendlyName || display.gdiName}
                  </option>
                ))}
              </select>
            </label>
          </div>
          <p className="text-xs text-muted">{t("enhancers.zoom.magnifierNote")}</p>
        </CardContent>
      </Card>
    </div>
  );
}
