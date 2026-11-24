import { Check, ImagePlus, Trash2 } from "lucide-react";
import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { PageHeader } from "@/components/common/PageHeader";
import { ParamSlider } from "@/components/common/ParamSlider";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { Accent } from "@/lib/bindings/Accent";
import type { AppearanceSettings } from "@/lib/bindings/AppearanceSettings";
import type { ErrorCode } from "@/lib/bindings/ErrorCode";
import type { Theme } from "@/lib/bindings/Theme";
import { ipc, isIpcError } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { saveSettings, useSettingsStore } from "@/state/settings";

const THEMES: Theme[] = ["black", "graphite"];
const ACCENTS: Accent[] = ["lime", "cyan", "blue", "violet", "pink", "red", "orange", "yellow"];
/** Igual que `UI_SCALES` en Rust. */
const SCALES = [90, 100, 110, 125] as const;
/** Igual que `MAX_BACKGROUND_BYTES` y `MAX_BACKGROUND_BLUR` en Rust. */
const MAX_BYTES = 25 * 1024 * 1024;
const MAX_BLUR = 24;
const ACCEPT = "image/png,image/jpeg,image/gif,image/webp";

const choice = (selected: boolean) =>
  cn(
    "flex items-center gap-3 rounded-xl border p-3 text-left transition-colors",
    selected ? "border-accent bg-accent/10" : "border-border hover:bg-surface-2",
  );

/** Pestaña Personalizar: tema, color de acento, tamaño y fondo propio. */
export function CustomizePage() {
  const { t } = useTranslation();
  const settings = useSettingsStore((store) => store.settings);
  const setSettings = useSettingsStore((store) => store.setSettings);
  const input = useRef<HTMLInputElement>(null);
  const [error, setError] = useState<ErrorCode | null>(null);
  const [uploading, setUploading] = useState(false);

  if (!settings) {
    return <PageHeader title={t("customize.title")} subtitle={t("customize.subtitle")} />;
  }
  const appearance = settings.appearance;
  const save = (patch: Partial<AppearanceSettings>) =>
    void saveSettings({ ...settings, appearance: { ...appearance, ...patch } });
  // Mientras se arrastra, solo cambia acá (se ve en vivo); al soltar, se guarda.
  const preview = (patch: Partial<AppearanceSettings>) =>
    setSettings({ ...settings, appearance: { ...appearance, ...patch } });

  const upload = (file: File) => {
    if (file.size > MAX_BYTES) {
      setError("backgroundInvalid");
      return;
    }
    setUploading(true);
    file
      .arrayBuffer()
      .then((buffer) => ipc.customizeBackgroundSet(new Uint8Array(buffer)))
      .then((next) => {
        setSettings(next);
        setError(null);
      })
      .catch((reason: unknown) => setError(isIpcError(reason) ? reason.code : "internal"))
      .finally(() => setUploading(false));
  };

  return (
    <div className="flex flex-col gap-6">
      <PageHeader title={t("customize.title")} subtitle={t("customize.subtitle")} />

      <Card>
        <CardHeader>
          <CardTitle>{t("customize.theme.title")}</CardTitle>
          <CardDescription>{t("customize.theme.description")}</CardDescription>
        </CardHeader>
        <CardContent className="grid gap-3 sm:grid-cols-2">
          {THEMES.map((theme) => {
            const selected = appearance.theme === theme;
            return (
              <button
                key={theme}
                type="button"
                aria-pressed={selected}
                onClick={() => save({ theme })}
                className={choice(selected)}
              >
                {/* Muestra con los colores del tema (los tokens cambian con data-theme). */}
                <span
                  data-theme={theme}
                  aria-hidden
                  className="flex h-12 w-16 shrink-0 flex-col justify-between overflow-hidden rounded-lg border border-border bg-bg p-1.5"
                >
                  <span className="h-2 w-8 rounded-sm bg-surface-2" />
                  <span className="h-2 w-5 rounded-sm bg-accent" />
                </span>
                <span className="flex flex-1 flex-col">
                  <span className="text-sm font-medium text-fg">
                    {t(`customize.theme.${theme}`)}
                  </span>
                  <span className="text-xs text-muted">{t(`customize.theme.${theme}Hint`)}</span>
                </span>
                {selected && <Check aria-hidden className="size-4 text-accent" />}
              </button>
            );
          })}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("customize.accent.title")}</CardTitle>
          <CardDescription>{t("customize.accent.description")}</CardDescription>
        </CardHeader>
        <CardContent className="grid grid-cols-2 gap-3 sm:grid-cols-4">
          {ACCENTS.map((accent) => {
            const selected = appearance.accent === accent;
            return (
              <button
                key={accent}
                type="button"
                aria-pressed={selected}
                onClick={() => save({ accent })}
                className={choice(selected)}
              >
                {/* Muestra con el color (el token cambia con data-accent). */}
                <span
                  data-accent={accent}
                  aria-hidden
                  className="size-6 shrink-0 rounded-full bg-accent ring-2 ring-accent/30"
                />
                <span className="flex-1 text-sm font-medium text-fg">
                  {t(`customize.accent.${accent}`)}
                </span>
                {selected && <Check aria-hidden className="size-4 text-accent" />}
              </button>
            );
          })}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("customize.size.title")}</CardTitle>
          <CardDescription>{t("customize.size.description")}</CardDescription>
        </CardHeader>
        <CardContent className="grid grid-cols-2 gap-3 sm:grid-cols-4">
          {SCALES.map((scale) => {
            const selected = appearance.uiScale === scale;
            return (
              <button
                key={scale}
                type="button"
                aria-pressed={selected}
                onClick={() => save({ uiScale: scale })}
                className={cn(choice(selected), "flex-col items-start gap-1")}
              >
                <span className="text-sm font-medium text-fg">{t(`customize.size.s${scale}`)}</span>
                <span className="text-xs text-muted">
                  {t("customize.units.percent", { value: scale })}
                </span>
              </button>
            );
          })}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("customize.background.title")}</CardTitle>
          <CardDescription>{t("customize.background.description")}</CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <div className="flex flex-wrap items-center gap-2">
            <input
              ref={input}
              type="file"
              accept={ACCEPT}
              aria-label={t("customize.background.choose")}
              className="sr-only"
              onChange={(event) => {
                const file = event.target.files?.[0];
                event.target.value = "";
                if (file) upload(file);
              }}
            />
            <Button variant="primary" disabled={uploading} onClick={() => input.current?.click()}>
              <ImagePlus aria-hidden />
              {uploading
                ? t("customize.background.uploading")
                : appearance.background
                  ? t("customize.background.replace")
                  : t("customize.background.choose")}
            </Button>
            {appearance.background && (
              <Button
                variant="ghost"
                onClick={() => {
                  ipc
                    .customizeBackgroundClear()
                    .then(setSettings)
                    .catch((reason: unknown) =>
                      setError(isIpcError(reason) ? reason.code : "internal"),
                    );
                }}
              >
                <Trash2 aria-hidden />
                {t("customize.background.remove")}
              </Button>
            )}
          </div>
          {error && (
            <p role="alert" className="text-sm text-warning">
              {t(`errors.${error}`)}
            </p>
          )}
          {appearance.background && (
            <div className="grid gap-5 md:grid-cols-2">
              <ParamSlider
                id="background-opacity"
                label={t("customize.background.opacity")}
                value={appearance.backgroundOpacity}
                range={{ min: 0, max: 100, default: 25, step: 1 }}
                display={t("customize.units.percent", {
                  value: Math.round(appearance.backgroundOpacity),
                })}
                resetTitle={t("display.resetParam")}
                onChange={(backgroundOpacity) => preview({ backgroundOpacity })}
                onCommit={(backgroundOpacity) => save({ backgroundOpacity })}
              />
              <ParamSlider
                id="background-blur"
                label={t("customize.background.blur")}
                value={appearance.backgroundBlur}
                range={{ min: 0, max: MAX_BLUR, default: 0, step: 1 }}
                display={t("customize.units.px", { value: Math.round(appearance.backgroundBlur) })}
                resetTitle={t("display.resetParam")}
                onChange={(backgroundBlur) => preview({ backgroundBlur })}
                onCommit={(backgroundBlur) => save({ backgroundBlur })}
              />
            </div>
          )}
          <p className="text-xs text-muted">{t("customize.background.note")}</p>
        </CardContent>
      </Card>
    </div>
  );
}
