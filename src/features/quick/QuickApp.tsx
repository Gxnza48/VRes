import { ExternalLink, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { ParamSlider } from "@/components/common/ParamSlider";
import { Button } from "@/components/ui/button";
import type { Look } from "@/lib/bindings/Look";
import ranges from "@/lib/bindings/ranges.json";
import type { Requested } from "@/lib/bindings/Requested";
import { events } from "@/lib/events";
import { inTauri, ipc } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useTheme } from "@/app/theme";
import { useCatalogStore } from "@/state/catalog";
import { useSettingsStore } from "@/state/settings";

/**
 * Popup rápido: una ventana chica junto a la bandeja con lo más usado
 * (color, saturación, vibrance y looks). Se cierra al hacer clic afuera.
 */
export function QuickApp() {
  const { t } = useTranslation();
  const [requested, setRequested] = useState<Requested | null>(null);
  const settings = useSettingsStore((store) => store.settings);
  const { lookPresets, load } = useCatalogStore();
  const hadFocus = useRef(false);
  useTheme();

  useEffect(() => {
    if (!inTauri()) return;
    load();
    ipc.lookGet().then(setRequested).catch(() => undefined);
    ipc
      .settingsGet()
      .then((value) => useSettingsStore.getState().setSettings(value))
      .catch(() => undefined);
    const subscriptions = [
      events.onRequestedChanged(setRequested),
      events.onSettingsChanged((value) => useSettingsStore.getState().setSettings(value)),
      // Al perder el foco (clic afuera) se cierra, pero solo si ya lo tuvo.
      events.onFocusChanged((focused) => {
        if (focused) hadFocus.current = true;
        else if (hadFocus.current) void ipc.quickClose();
      }),
    ];
    requestAnimationFrame(() =>
      requestAnimationFrame(() => {
        ipc.appUiReady().catch(() => undefined);
      }),
    );
    return () => {
      for (const subscription of subscriptions) {
        void subscription.then((unlisten) => unlisten());
      }
    };
  }, [load]);

  if (!requested) {
    return <div className="h-full rounded-card border border-border bg-bg" />;
  }
  const look = requested.look;
  const enabled = requested.colorEnabled;
  const commit = (next: Look) => {
    ipc.lookCommit(next).then(setRequested).catch(() => undefined);
  };
  const preview = (next: Look) => {
    setRequested({ ...requested, look: next });
    ipc.lookPreview(next).catch(() => undefined);
  };
  const favorites = (settings?.library.looks ?? []).filter((item) => item.favorite);
  const chips = [
    ...favorites.map((item) => ({ key: item.id, name: item.name, look: item.look })),
    ...lookPresets
      .filter((preset) => preset.game === null)
      .map((preset) => ({ key: preset.id, name: t(`looks.presets.${preset.id}`), look: preset.look })),
  ];

  return (
    <div className="flex h-full flex-col gap-4 overflow-hidden rounded-card border border-accent/40 bg-bg p-5">
      <header className="flex items-center justify-between">
        <img src="/vres-logo.png" alt={t("app.name")} className="h-6" />
        <Button size="sm" variant="ghost" aria-label={t("quick.close")} onClick={() => void ipc.quickClose()}>
          <X aria-hidden />
        </Button>
      </header>

      <Button
        variant={enabled ? "primary" : "secondary"}
        onClick={() =>
          ipc
            .lookSetEnabled(!enabled)
            .then(setRequested)
            .catch(() => undefined)
        }
      >
        {t("display.enabled")}: {enabled ? t("display.on") : t("display.off")}
      </Button>

      <div className="flex flex-col gap-1">
        <div className="flex items-baseline justify-between">
          <span className="text-sm text-muted">{t("display.saturation.title")}</span>
          <span className="font-display text-2xl font-black text-accent tabular-nums">
            {Math.round(look.saturation)}
          </span>
        </div>
        <ParamSlider
          id="quick-saturation"
          label={t("display.saturation.title")}
          hideHeader
          value={look.saturation}
          range={ranges.saturation}
          display={String(Math.round(look.saturation))}
          disabled={!enabled}
          resetTitle={t("display.resetParam")}
          onChange={(value) => preview({ ...look, saturation: value })}
          onCommit={(value) => commit({ ...look, saturation: value })}
        />
      </div>

      <div className="flex flex-col gap-1">
        <div className="flex items-baseline justify-between">
          <span className="text-sm text-muted">{t("display.vibrance.title")}</span>
          <span className="font-display text-2xl font-black text-accent tabular-nums">
            {Math.round(look.vibrance)}
          </span>
        </div>
        <ParamSlider
          id="quick-vibrance"
          label={t("display.vibrance.title")}
          hideHeader
          value={look.vibrance}
          range={ranges.vibrance}
          display={String(Math.round(look.vibrance))}
          disabled={!enabled}
          resetTitle={t("display.resetParam")}
          onChange={(value) => preview({ ...look, vibrance: value })}
          onCommit={(value) => commit({ ...look, vibrance: value })}
        />
      </div>

      <div className="flex min-h-0 flex-1 flex-col gap-2">
        <span className="text-sm text-muted">{t("quick.looks")}</span>
        <div className="flex flex-wrap gap-1.5 overflow-y-auto">
          {chips.map((chip) => (
            <button
              key={chip.key}
              type="button"
              disabled={!enabled}
              onClick={() => commit(chip.look)}
              className={cn(
                "rounded-lg border border-border bg-surface-2 px-2.5 py-1 text-xs text-fg transition-colors",
                "hover:border-accent disabled:opacity-50",
              )}
            >
              {chip.name}
            </button>
          ))}
        </div>
      </div>

      <Button
        size="sm"
        variant="ghost"
        onClick={() => {
          void ipc.appOpenMain();
          void ipc.quickClose();
        }}
      >
        <ExternalLink aria-hidden />
        {t("quick.openMain")}
      </Button>
    </div>
  );
}
