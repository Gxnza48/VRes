import { Repeat } from "lucide-react";
import { useCallback } from "react";
import { useTranslation } from "react-i18next";

import { HotkeyRecorder } from "@/components/common/HotkeyRecorder";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { DisplayInfo } from "@/lib/bindings/DisplayInfo";
import type { Hotkey } from "@/lib/bindings/Hotkey";
import type { Settings } from "@/lib/bindings/Settings";
import { ipc } from "@/lib/ipc";
import { useResolutionStore } from "@/state/resolution";
import { saveSettings as persist } from "@/state/settings";

import { sizeLabel } from "./format";

const CONFIRM_CHOICES = [10, 15, 30] as const;

/** Guarda y muestra el error (por ejemplo, un atajo ocupado) en la pestaña. */
async function saveSettings(settings: Settings): Promise<void> {
  const store = useResolutionStore.getState();
  store.setError(null);
  const error = await persist(settings);
  if (error) store.setError(error);
}

interface HotkeyCardProps {
  settings: Settings;
  displays: DisplayInfo[];
  busy: boolean;
}

/** Atajo nativa ↔ estirada, tonos y segundos para confirmar. */
export function HotkeyCard({ settings, displays, busy }: HotkeyCardProps) {
  const { t } = useTranslation();
  const target = settings.resolution.toggle;
  const targetDisplay = target ? displays.find((display) => display.id === target.displayId) : null;

  const onHotkeyChange = useCallback(
    (hotkey: Hotkey | null) => {
      void saveSettings({
        ...settings,
        hotkeys: { ...settings.hotkeys, resolutionToggle: hotkey },
      });
    },
    [settings],
  );

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("resolution.hotkey.title")}</CardTitle>
        <CardDescription>{t("resolution.hotkey.description")}</CardDescription>
      </CardHeader>
      <CardContent className="gap-4">
        <HotkeyRecorder
          value={settings.hotkeys.resolutionToggle}
          disabled={busy}
          onChange={onHotkeyChange}
        />

        {target ? (
          <div className="flex flex-wrap items-center gap-3">
            <p className="text-sm text-fg">
              {t("resolution.hotkey.target", {
                mode: `${sizeLabel(target.mode.width, target.mode.height)} (${t(`resolution.scaling.${target.mode.scaling}`)})`,
                display: targetDisplay?.friendlyName || targetDisplay?.gdiName || "—",
              })}
            </p>
            <Button size="sm" disabled={busy} onClick={() => void ipc.resolutionToggle()}>
              <Repeat aria-hidden />
              {t("resolution.hotkey.toggleNow")}
            </Button>
          </div>
        ) : (
          <p className="text-sm text-muted">{t("resolution.hotkey.noTarget")}</p>
        )}

        <div className="flex flex-wrap items-center gap-6 text-sm">
          <label className="flex items-center gap-2 text-fg">
            <input
              type="checkbox"
              checked={settings.resolution.tones}
              onChange={(event) =>
                void saveSettings({
                  ...settings,
                  resolution: { ...settings.resolution, tones: event.target.checked },
                })
              }
              className="size-4 accent-accent"
            />
            {t("resolution.hotkey.tones")}
          </label>
          <label className="flex items-center gap-2 text-muted">
            {t("resolution.confirm.title")}
            <select
              value={settings.resolution.confirmSeconds}
              onChange={(event) =>
                void saveSettings({
                  ...settings,
                  resolution: { ...settings.resolution, confirmSeconds: Number(event.target.value) },
                })
              }
              className="rounded-lg border border-border bg-surface-2 px-2 py-1 text-fg"
            >
              {CONFIRM_CHOICES.map((seconds) => (
                <option key={seconds} value={seconds}>
                  {t("resolution.confirm.seconds", { count: seconds })}
                </option>
              ))}
            </select>
          </label>
        </div>
      </CardContent>
    </Card>
  );
}
