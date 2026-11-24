import { useTranslation } from "react-i18next";

import { ParamSlider } from "@/components/common/ParamSlider";
import { ShareBox } from "@/components/common/ShareBox";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { Look } from "@/lib/bindings/Look";
import type { LumaFormula } from "@/lib/bindings/LumaFormula";
import ranges from "@/lib/bindings/ranges.json";
import type { VisionMode } from "@/lib/bindings/VisionMode";
import { ipc } from "@/lib/ipc";

import { commitLook, previewLook } from "./actions";

const LUMAS: LumaFormula[] = ["rec709", "rec601", "rec2020", "average"];
const VISIONS: VisionMode[] = ["off", "protan", "deutan", "tritan"];
const field = "rounded-lg border border-border bg-surface-2 px-2 py-1.5 text-sm text-fg";

interface AdvancedColorProps {
  look: Look;
  enabled: boolean;
}

/** Profundidad de color, visión de color, cuidado visual y compartir el look. */
export function AdvancedColor({ look, enabled }: AdvancedColorProps) {
  const { t } = useTranslation();
  const slider = (
    id: string,
    label: string,
    value: number,
    range: { min: number; max: number; default: number; step: number },
    unit: "deg" | "percent",
    build: (value: number) => Look,
  ) => (
    <ParamSlider
      key={id}
      id={id}
      label={label}
      value={value}
      range={range}
      display={t(`crosshair.units.${unit}`, { value: Math.round(value) })}
      disabled={!enabled}
      resetTitle={t("display.resetParam")}
      onChange={(next) => previewLook(build(next))}
      onCommit={(next) => void commitLook(build(next))}
    />
  );

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle>{t("display.depth.title")}</CardTitle>
          <CardDescription>{t("display.depth.description")}</CardDescription>
        </CardHeader>
        <CardContent className="gap-5">
          <label className="flex flex-wrap items-center gap-3 text-sm text-muted">
            {t("display.depth.luma")}
            <select
              value={look.color.luma}
              disabled={!enabled}
              onChange={(event) =>
                void commitLook({
                  ...look,
                  color: { ...look.color, luma: event.target.value as LumaFormula },
                })
              }
              className={field}
            >
              {LUMAS.map((luma) => (
                <option key={luma} value={luma}>
                  {t(`display.depth.lumas.${luma}`)}
                </option>
              ))}
            </select>
          </label>
          <div className="grid gap-x-8 gap-y-5 md:grid-cols-2">
            {slider("hue-shift", t("display.depth.hueShift"), look.color.hueShiftDeg, ranges.hueShift, "deg", (value) => ({ ...look, color: { ...look.color, hueShiftDeg: value } }))}
            {slider("hue-amount", t("display.depth.hueAmount"), look.color.hueShiftAmount, ranges.percent, "percent", (value) => ({ ...look, color: { ...look.color, hueShiftAmount: value } }))}
            {slider("darken-hue", t("display.depth.darkenHue"), look.color.darkenHueDeg, ranges.hue, "deg", (value) => ({ ...look, color: { ...look.color, darkenHueDeg: value } }))}
            {slider("darken-amount", t("display.depth.darkenAmount"), look.color.darkenAmount, ranges.percent, "percent", (value) => ({ ...look, color: { ...look.color, darkenAmount: value } }))}
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("display.vision.title")}</CardTitle>
          <CardDescription>{t("display.vision.description")}</CardDescription>
        </CardHeader>
        <CardContent className="gap-5">
          <label className="flex flex-wrap items-center gap-3 text-sm text-muted">
            {t("display.vision.mode")}
            <select
              value={look.vision.mode}
              disabled={!enabled}
              onChange={(event) =>
                void commitLook({
                  ...look,
                  vision: { ...look.vision, mode: event.target.value as VisionMode },
                })
              }
              className={field}
            >
              {VISIONS.map((mode) => (
                <option key={mode} value={mode}>
                  {t(`display.vision.modes.${mode}`)}
                </option>
              ))}
            </select>
          </label>
          {look.vision.mode !== "off" &&
            slider("vision-strength", t("display.vision.strength"), look.vision.strength, ranges.percent, "percent", (value) => ({ ...look, vision: { ...look.vision, strength: value } }))}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("display.eyeCare.title")}</CardTitle>
          <CardDescription>{t("display.eyeCare.description")}</CardDescription>
        </CardHeader>
        <CardContent className="grid gap-x-8 gap-y-5 md:grid-cols-2">
          {slider("eye-warmth", t("display.eyeCare.warmth"), look.eyeCare.warmth, ranges.percent, "percent", (value) => ({ ...look, eyeCare: { ...look.eyeCare, warmth: value } }))}
          {slider("eye-dim", t("display.eyeCare.dim"), look.eyeCare.dim, ranges.dim, "percent", (value) => ({ ...look, eyeCare: { ...look.eyeCare, dim: value } }))}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("share.lookTitle")}</CardTitle>
          <CardDescription>{t("share.lookDescription")}</CardDescription>
        </CardHeader>
        <CardContent>
          <ShareBox
            kind="look"
            encode={() => ipc.shareEncodeLook(look)}
            onDecoded={(decoded) => {
              if (decoded.look) void commitLook(decoded.look);
            }}
          />
        </CardContent>
      </Card>
    </>
  );
}
