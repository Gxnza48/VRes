import { Crosshair as CrosshairIcon, ExternalLink, ImageUp, Save, Shapes, Trash2 } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { HotkeyRecorder } from "@/components/common/HotkeyRecorder";
import { PageHeader } from "@/components/common/PageHeader";
import { ParamSlider, type SliderRange } from "@/components/common/ParamSlider";
import { ShareBox } from "@/components/common/ShareBox";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { Crosshair } from "@/lib/bindings/Crosshair";
import type { CrosshairPreset } from "@/lib/bindings/CrosshairPreset";
import type { CrosshairSettings } from "@/lib/bindings/CrosshairSettings";
import type { ErrorCode } from "@/lib/bindings/ErrorCode";
import type { Hotkey } from "@/lib/bindings/Hotkey";
import ranges from "@/lib/bindings/ranges.json";
import { ipc } from "@/lib/ipc";
import { CROSSHAIRS_URL, openSite } from "@/lib/links";
import { cn } from "@/lib/utils";
import { useEngineStore } from "@/state/engine";
import { saveSettings, useSettingsStore } from "@/state/settings";

import { CrosshairCanvas } from "./CrosshairCanvas";
import { imageToPngDataUrl } from "./image";

type Mode = "preview" | "commit";

const field = "rounded-lg border border-border bg-surface-2 px-2 py-1.5 text-sm text-fg";

/** Un interruptor con su etiqueta. */
function Toggle({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label className="flex items-center gap-2 text-sm text-fg">
      <input
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
        className="size-4 accent-accent"
      />
      {label}
    </label>
  );
}

/** Pestaña Crosshair: presets, editor por partes, imagen propia y guardados. */
export function CrosshairPage() {
  const { t } = useTranslation();
  const settings = useSettingsStore((store) => store.settings);
  const displays = useEngineStore((store) => store.displays);
  const [draft, setDraft] = useState<Crosshair | null>(null);
  const [presets, setPresets] = useState<CrosshairPreset[]>([]);
  const [error, setError] = useState<ErrorCode | null>(null);
  const [hotkeyError, setHotkeyError] = useState<ErrorCode | null>(null);
  const [imageError, setImageError] = useState(false);
  const [name, setName] = useState("");
  const frame = useRef<number | null>(null);
  const pendingPreview = useRef<Crosshair | null>(null);
  const colorTimer = useRef<number | null>(null);

  useEffect(() => {
    ipc
      .crosshairPresets()
      .then(setPresets)
      .catch((reason: unknown) => console.error("crosshair_presets", reason));
  }, []);

  const config = settings?.crosshair ?? null;
  const current = draft ?? config?.current ?? null;

  const saveCrosshair = useCallback(
    async (patch: Partial<CrosshairSettings>) => {
      const latest = useSettingsStore.getState().settings;
      if (!latest) return;
      setError(null);
      const code = await saveSettings({
        ...latest,
        crosshair: { ...latest.crosshair, ...patch },
      });
      if (code) setError(code);
      setDraft(null);
    },
    [],
  );

  // El atajo vive con los demás atajos; si otro programa lo usa, no se guarda.
  const onHotkeyChange = useCallback((hotkey: Hotkey | null) => {
    const latest = useSettingsStore.getState().settings;
    if (!latest) return;
    setHotkeyError(null);
    void saveSettings({
      ...latest,
      hotkeys: { ...latest.hotkeys, crosshairToggle: hotkey },
    }).then((code) => {
      if (code) setHotkeyError(code);
    });
  }, []);

  const change = useCallback(
    (next: Crosshair, mode: Mode) => {
      setDraft(next);
      if (mode === "commit") {
        void saveCrosshair({ current: next });
        return;
      }
      // Vista previa en pantalla: como mucho una por cuadro.
      pendingPreview.current = next;
      if (frame.current !== null) return;
      frame.current = requestAnimationFrame(() => {
        frame.current = null;
        const value = pendingPreview.current;
        pendingPreview.current = null;
        if (value) ipc.crosshairPreview(value).catch(() => undefined);
      });
    },
    [saveCrosshair],
  );

  // El selector de color dispara cambios seguidos: se guarda al rato de soltarlo.
  const changeColor = useCallback(
    (next: Crosshair) => {
      change(next, "preview");
      if (colorTimer.current !== null) window.clearTimeout(colorTimer.current);
      colorTimer.current = window.setTimeout(() => change(next, "commit"), 400);
    },
    [change],
  );

  if (!config || !current) {
    return (
      <div className="flex flex-col gap-6">
        <PageHeader title={t("crosshair.title")} subtitle={t("crosshair.subtitle")} />
      </div>
    );
  }

  const slider = (
    id: string,
    label: string,
    value: number,
    range: SliderRange,
    unit: "px" | "deg" | "percent",
    build: (value: number) => Crosshair,
  ) => (
    <ParamSlider
      key={id}
      id={`crosshair-${id}`}
      label={label}
      value={value}
      range={range}
      display={t(`crosshair.units.${unit}`, { value: Math.round(value) })}
      resetTitle={t("display.resetParam")}
      onChange={(next) => change(build(next), "preview")}
      onCommit={(next) => change(build(next), "commit")}
    />
  );

  const isImage = current.kind === "image";
  const hotkeys = settings?.hotkeys;

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        title={t("crosshair.title")}
        subtitle={t("crosshair.subtitle")}
        aside={
          <Button
            variant={config.enabled ? "primary" : "secondary"}
            onClick={() => void saveCrosshair({ enabled: !config.enabled })}
          >
            <CrosshairIcon aria-hidden />
            {t("crosshair.enabled")}: {config.enabled ? t("crosshair.on") : t("crosshair.off")}
          </Button>
        }
      />
      <p className="rounded-xl border border-warning/40 bg-warning/10 px-4 py-3 text-sm text-fg">
        {t("crosshair.warning")}
      </p>
      {error && (
        <p role="alert" className="text-sm text-danger">
          {t(`errors.${error}`)}
        </p>
      )}

      <Card>
        <CardHeader>
          <CardTitle>{t("crosshair.preview.title")}</CardTitle>
          <CardDescription>{t("crosshair.preview.description")}</CardDescription>
        </CardHeader>
        <CardContent className="flex-row flex-wrap items-center gap-6">
          <CrosshairCanvas crosshair={current} box={160} label={t("crosshair.preview.title")} />
          <CrosshairCanvas crosshair={current} box={160} zoom={4} label={t("crosshair.preview.zoom")} />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("crosshair.hotkey.title")}</CardTitle>
          <CardDescription>{t("crosshair.hotkey.description")}</CardDescription>
        </CardHeader>
        <CardContent className="gap-3">
          <HotkeyRecorder
            value={hotkeys?.crosshairToggle ?? null}
            others={hotkeys?.resolutionToggle ? [hotkeys.resolutionToggle] : []}
            onChange={onHotkeyChange}
          />
          {hotkeyError && (
            <p role="alert" className="text-sm text-danger">
              {t(`errors.${hotkeyError}`)}
            </p>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("crosshair.presets.title")}</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-4 gap-3 md:grid-cols-6 lg:grid-cols-8">
            {presets.map((preset) => (
              <button
                key={preset.id}
                type="button"
                onClick={() => change(preset.crosshair, "commit")}
                className="flex flex-col items-center gap-2 rounded-xl border border-border bg-surface-2 p-2 text-xs text-muted transition-colors hover:border-accent hover:text-fg"
              >
                <CrosshairCanvas crosshair={preset.crosshair} box={56} zoom={2} className="border-0" />
                {t(`crosshair.presets.${preset.id}`)}
              </button>
            ))}
          </div>
        </CardContent>
      </Card>

      <Card className="flex flex-row flex-wrap items-center justify-between gap-4">
        <div className="flex flex-col gap-1">
          <CardTitle>{t("crosshair.browse.title")}</CardTitle>
          <CardDescription>{t("crosshair.browse.description")}</CardDescription>
        </div>
        <Button variant="primary" onClick={() => openSite(CROSSHAIRS_URL)}>
          <ExternalLink aria-hidden />
          {t("crosshair.browse.open")}
        </Button>
      </Card>

      {!isImage && (
        <>
          <Card>
            <CardHeader>
              <CardTitle>{t("crosshair.color.title")}</CardTitle>
            </CardHeader>
            <CardContent className="gap-4">
              <label className="flex items-center gap-3 text-sm text-fg">
                <input
                  type="color"
                  value={current.color}
                  onChange={(event) => changeColor({ ...current, color: event.target.value })}
                  className="h-9 w-14 cursor-pointer rounded-lg border border-border bg-surface-2"
                />
                {t("crosshair.color.fill")}
              </label>
              {slider("opacity", t("crosshair.color.opacity"), current.opacity, ranges.crosshairOpacity, "percent", (value) => ({ ...current, opacity: value }))}
            </CardContent>
          </Card>

          <Card>
            <CardHeader className="flex-row items-center justify-between">
              <CardTitle>{t("crosshair.arms.title")}</CardTitle>
              <Toggle
                label={t("crosshair.arms.enabled")}
                checked={current.arms.enabled}
                onChange={(enabled) => change({ ...current, arms: { ...current.arms, enabled } }, "commit")}
              />
            </CardHeader>
            {current.arms.enabled && (
              <CardContent className="gap-4">
                <div className="flex flex-wrap gap-4">
                  {(["top", "right", "bottom", "left"] as const).map((side) => (
                    <Toggle
                      key={side}
                      label={t(`crosshair.arms.${side}`)}
                      checked={current.arms[side]}
                      onChange={(on) => change({ ...current, arms: { ...current.arms, [side]: on } }, "commit")}
                    />
                  ))}
                </div>
                <div className="grid gap-x-8 gap-y-5 md:grid-cols-3">
                  {slider("length", t("crosshair.arms.length"), current.arms.length, ranges.crosshairLength, "px", (value) => ({ ...current, arms: { ...current.arms, length: value } }))}
                  {slider("thickness", t("crosshair.arms.thickness"), current.arms.thickness, ranges.crosshairThickness, "px", (value) => ({ ...current, arms: { ...current.arms, thickness: value } }))}
                  {slider("gap", t("crosshair.arms.gap"), current.arms.gap, ranges.crosshairGap, "px", (value) => ({ ...current, arms: { ...current.arms, gap: value } }))}
                </div>
              </CardContent>
            )}
          </Card>

          <Card>
            <CardHeader className="flex-row items-center justify-between">
              <CardTitle>{t("crosshair.dot.title")}</CardTitle>
              <Toggle
                label={t("crosshair.dot.enabled")}
                checked={current.dot.enabled}
                onChange={(enabled) => change({ ...current, dot: { ...current.dot, enabled } }, "commit")}
              />
            </CardHeader>
            {current.dot.enabled && (
              <CardContent className="gap-4">
                <Toggle
                  label={t("crosshair.dot.round")}
                  checked={current.dot.round}
                  onChange={(round) => change({ ...current, dot: { ...current.dot, round } }, "commit")}
                />
                {slider("dot", t("crosshair.dot.size"), current.dot.size, ranges.crosshairDot, "px", (value) => ({ ...current, dot: { ...current.dot, size: value } }))}
              </CardContent>
            )}
          </Card>

          <Card>
            <CardHeader className="flex-row items-center justify-between">
              <CardTitle>{t("crosshair.ring.title")}</CardTitle>
              <Toggle
                label={t("crosshair.ring.enabled")}
                checked={current.ring.enabled}
                onChange={(enabled) => change({ ...current, ring: { ...current.ring, enabled } }, "commit")}
              />
            </CardHeader>
            {current.ring.enabled && (
              <CardContent className="grid gap-x-8 gap-y-5 md:grid-cols-2">
                {slider("radius", t("crosshair.ring.radius"), current.ring.radius, ranges.crosshairRadius, "px", (value) => ({ ...current, ring: { ...current.ring, radius: value } }))}
                {slider("ring", t("crosshair.ring.thickness"), current.ring.thickness, ranges.crosshairRing, "px", (value) => ({ ...current, ring: { ...current.ring, thickness: value } }))}
              </CardContent>
            )}
          </Card>

          <Card>
            <CardHeader className="flex-row items-center justify-between">
              <CardTitle>{t("crosshair.outline.title")}</CardTitle>
              <Toggle
                label={t("crosshair.outline.enabled")}
                checked={current.outline.enabled}
                onChange={(enabled) => change({ ...current, outline: { ...current.outline, enabled } }, "commit")}
              />
            </CardHeader>
            {current.outline.enabled && (
              <CardContent className="gap-4">
                <label className="flex items-center gap-3 text-sm text-fg">
                  <input
                    type="color"
                    value={current.outline.color}
                    onChange={(event) =>
                      changeColor({ ...current, outline: { ...current.outline, color: event.target.value } })
                    }
                    className="h-9 w-14 cursor-pointer rounded-lg border border-border bg-surface-2"
                  />
                  {t("crosshair.outline.color")}
                </label>
                <div className="grid gap-x-8 gap-y-5 md:grid-cols-2">
                  {slider("outline", t("crosshair.outline.thickness"), current.outline.thickness, ranges.crosshairOutline, "px", (value) => ({ ...current, outline: { ...current.outline, thickness: value } }))}
                  {slider("outline-opacity", t("crosshair.outline.opacity"), current.outline.opacity, ranges.crosshairOpacity, "percent", (value) => ({ ...current, outline: { ...current.outline, opacity: value } }))}
                </div>
              </CardContent>
            )}
          </Card>
        </>
      )}

      <Card>
        <CardHeader>
          <CardTitle>{t("crosshair.transform.title")}</CardTitle>
        </CardHeader>
        <CardContent className="grid gap-x-8 gap-y-5 md:grid-cols-2">
          {slider("rotation", t("crosshair.transform.rotation"), current.rotation, ranges.crosshairRotation, "deg", (value) => ({ ...current, rotation: value }))}
          {slider("scale", t("crosshair.transform.scale"), current.scale, ranges.crosshairScale, "percent", (value) => ({ ...current, scale: value }))}
          {slider("offset-x", t("crosshair.transform.offsetX"), current.offsetX, ranges.crosshairOffset, "px", (value) => ({ ...current, offsetX: Math.round(value) }))}
          {slider("offset-y", t("crosshair.transform.offsetY"), current.offsetY, ranges.crosshairOffset, "px", (value) => ({ ...current, offsetY: Math.round(value) }))}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("crosshair.image.title")}</CardTitle>
          <CardDescription>{t("crosshair.image.hint")}</CardDescription>
        </CardHeader>
        <CardContent className="flex-row flex-wrap items-center gap-3">
          <label className={cn(field, "inline-flex cursor-pointer items-center gap-2 hover:border-accent")}>
            <ImageUp aria-hidden className="size-4" />
            {t("crosshair.image.choose")}
            <input
              type="file"
              accept="image/png,image/gif,image/webp,image/jpeg"
              className="sr-only"
              onChange={(event) => {
                const file = event.target.files?.[0];
                event.target.value = "";
                if (!file) return;
                setImageError(false);
                imageToPngDataUrl(file)
                  .then((image) => change({ ...current, kind: "image", image }, "commit"))
                  .catch(() => setImageError(true));
              }}
            />
          </label>
          {isImage && (
            <Button size="sm" onClick={() => change({ ...current, kind: "vector" }, "commit")}>
              <Shapes aria-hidden />
              {t("crosshair.image.useShapes")}
            </Button>
          )}
          {imageError && <p className="text-sm text-danger">{t("crosshair.image.invalid")}</p>}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("crosshair.placement.title")}</CardTitle>
        </CardHeader>
        <CardContent className="gap-4">
          <label className="flex flex-wrap items-center gap-3 text-sm text-muted">
            {t("crosshair.placement.display")}
            <select
              value={config.displayId ?? ""}
              onChange={(event) => void saveCrosshair({ displayId: event.target.value || null })}
              className={field}
            >
              <option value="">{t("crosshair.placement.primary")}</option>
              {displays.map((display, index) => (
                <option key={display.id} value={display.id}>
                  {index + 1} · {display.friendlyName || display.gdiName}
                </option>
              ))}
            </select>
          </label>
          <div className="flex flex-col gap-1">
            <Toggle
              label={t("crosshair.placement.hideFromCapture")}
              checked={config.hideFromCapture}
              onChange={(hideFromCapture) => void saveCrosshair({ hideFromCapture })}
            />
            <p className="text-xs text-muted">{t("crosshair.placement.hideHint")}</p>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("share.crosshairTitle")}</CardTitle>
          <CardDescription>{t("share.crosshairDescription")}</CardDescription>
        </CardHeader>
        <CardContent>
          <ShareBox
            kind="crosshair"
            encode={() => ipc.shareEncodeCrosshair(current)}
            onDecoded={(decoded) => {
              if (decoded.crosshair) change(decoded.crosshair, "commit");
            }}
          />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("crosshair.saved.title")}</CardTitle>
        </CardHeader>
        <CardContent className="gap-4">
          <form
            className="flex flex-wrap items-center gap-2"
            onSubmit={(event) => {
              event.preventDefault();
              const trimmed = name.trim();
              if (!trimmed) return;
              void saveCrosshair({
                current,
                saved: [{ name: trimmed, crosshair: current }, ...config.saved],
              });
              setName("");
            }}
          >
            <input
              value={name}
              maxLength={40}
              placeholder={t("crosshair.saved.name")}
              aria-label={t("crosshair.saved.name")}
              onChange={(event) => setName(event.target.value)}
              className={cn(field, "w-56")}
            />
            <Button type="submit" variant="primary" disabled={!name.trim()}>
              <Save aria-hidden />
              {t("crosshair.saved.save")}
            </Button>
          </form>
          {config.saved.length === 0 ? (
            <p className="text-sm text-muted">{t("crosshair.saved.empty")}</p>
          ) : (
            <ul className="grid gap-3 md:grid-cols-2">
              {config.saved.map((item, index) => (
                <li
                  key={`${item.name}-${index}`}
                  className="flex items-center gap-3 rounded-xl border border-border bg-surface-2 p-2"
                >
                  <CrosshairCanvas crosshair={item.crosshair} box={48} zoom={2} className="border-0" />
                  <span className="flex-1 truncate text-sm text-fg">{item.name}</span>
                  <Button size="sm" onClick={() => change(item.crosshair, "commit")}>
                    {t("crosshair.saved.apply")}
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    aria-label={t("crosshair.saved.remove")}
                    onClick={() =>
                      void saveCrosshair({
                        saved: config.saved.filter((_, position) => position !== index),
                      })
                    }
                  >
                    <Trash2 aria-hidden />
                  </Button>
                </li>
              ))}
            </ul>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
