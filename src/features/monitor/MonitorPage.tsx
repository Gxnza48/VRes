import { RefreshCw, RotateCcw } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { PageHeader } from "@/components/common/PageHeader";
import { ParamSlider } from "@/components/common/ParamSlider";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { DdcControl } from "@/lib/bindings/DdcControl";
import type { DisplayInfo } from "@/lib/bindings/DisplayInfo";
import type { DisplayTrim } from "@/lib/bindings/DisplayTrim";
import type { ErrorCode } from "@/lib/bindings/ErrorCode";
import type { MonitorDetails } from "@/lib/bindings/MonitorDetails";
import ranges from "@/lib/bindings/ranges.json";
import { ipc, isIpcError } from "@/lib/ipc";
import { useEngineStore } from "@/state/engine";

const CONTROLS: DdcControl[] = ["brightness", "contrast", "redGain", "greenGain", "blueGain"];
const NEUTRAL_TRIM: DisplayTrim = { gamma: 1, warmth: 0, gain: [1, 1, 1] };
const VENDORS: Record<string, string> = { nvidia: "NVIDIA", amd: "AMD", intel: "Intel" };

function errorCode(error: unknown): ErrorCode {
  return isIpcError(error) ? error.code : "internal";
}

/** Pestaña Monitor: ficha de cada pantalla, ajustes por hardware y ajuste propio. */
export function MonitorPage() {
  const { t } = useTranslation();
  const displays = useEngineStore((store) => store.displays);
  const [details, setDetails] = useState<MonitorDetails[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<ErrorCode | null>(null);

  const load = useCallback(() => {
    ipc
      .monitorDetails()
      .then((list) => {
        setDetails(list);
        setError(null);
      })
      .catch((reason: unknown) => setError(errorCode(reason)))
      .finally(() => setLoading(false));
  }, []);
  useEffect(load, [load]);

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        title={t("monitor.title")}
        subtitle={t("monitor.subtitle")}
        aside={
          <Button
            variant="secondary"
            disabled={loading}
            onClick={() => {
              setLoading(true);
              load();
            }}
          >
            <RefreshCw aria-hidden />
            {loading ? t("monitor.reading") : t("monitor.refresh")}
          </Button>
        }
      />
      {error && (
        <p role="alert" className="text-sm text-warning">
          {t(`errors.${error}`)}
        </p>
      )}
      {displays.map((display) => (
        <MonitorCard
          key={display.id}
          display={display}
          details={details?.find((item) => item.displayId === display.id) ?? null}
          loading={loading}
          onRestored={() => {
            setLoading(true);
            load();
          }}
        />
      ))}
    </div>
  );
}

interface MonitorCardProps {
  display: DisplayInfo;
  details: MonitorDetails | null;
  loading: boolean;
  onRestored: () => void;
}

function MonitorCard({ display, details, loading, onRestored }: MonitorCardProps) {
  const { t } = useTranslation();
  const vibrance = useEngineStore((store) => store.status?.vibrance[display.id] ?? null);
  const [hardware, setHardware] = useState<MonitorDetails | null>(details);
  const [drafts, setDrafts] = useState<Record<string, number>>({});
  const [error, setError] = useState<ErrorCode | null>(null);
  const [busy, setBusy] = useState(false);

  // Los datos nuevos de la página reemplazan a los locales.
  const [seen, setSeen] = useState(details);
  if (details !== seen) {
    setSeen(details);
    setHardware(details);
  }

  const name = display.friendlyName || display.gdiName;
  const vendor = VENDORS[display.adapter.vendor];
  const draft = (id: string, value: number) => drafts[id] ?? value;
  const setDraft = (id: string, value: number | null) =>
    setDrafts((current) => {
      const next = { ...current };
      if (value === null) delete next[id];
      else next[id] = value;
      return next;
    });

  const writeDdc = (control: DdcControl, value: number) => {
    setBusy(true);
    ipc
      .ddcSet(display.id, control, value)
      .then((result) => {
        setHardware((current) =>
          current
            ? { ...current, ddc: { ...current.ddc, [control]: result }, hasInitial: true }
            : current,
        );
        setError(null);
      })
      .catch((reason: unknown) => setError(errorCode(reason)))
      .finally(() => {
        setDraft(control, null);
        setBusy(false);
      });
  };

  const writePanel = (value: number) => {
    setBusy(true);
    ipc
      .panelBrightnessSet(display.id, value)
      .then((result) => {
        setHardware((current) =>
          current ? { ...current, panelBrightness: result, hasInitial: true } : current,
        );
        setError(null);
      })
      .catch((reason: unknown) => setError(errorCode(reason)))
      .finally(() => {
        setDraft("panel", null);
        setBusy(false);
      });
  };

  const controls = CONTROLS.filter((control) => hardware?.ddc[control]);
  const hasHardware = controls.length > 0 || hardware?.panelBrightness != null;

  return (
    <Card>
      <CardHeader className="gap-3">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <CardTitle>{name}</CardTitle>
          <div className="flex flex-wrap gap-1.5">
            {display.primary && <Badge tone="accent">{t("monitor.primary")}</Badge>}
            <Badge>{display.internal ? t("monitor.internal") : t("monitor.external")}</Badge>
            {display.hdr.enabled && <Badge>{t("monitor.hdrOn")}</Badge>}
          </div>
        </div>
        <CardDescription className="flex flex-wrap gap-x-4 gap-y-1">
          <span>
            {display.bounds.width} × {display.bounds.height} · {Math.round(display.refreshHz)} Hz
          </span>
          {hardware?.scalePercent != null && (
            <span>{t("monitor.scale", { value: hardware.scalePercent })}</span>
          )}
          <span>
            {display.adapter.name || vendor || t("monitor.unknownGpu")}
          </span>
          <span>
            {vibrance ? t(`monitor.vibrance.${vibrance.backend}`) : t("monitor.vibrance.none")}
          </span>
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-6">
        <section className="flex flex-col gap-4">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <h3 className="text-sm font-medium text-fg">{t("monitor.hardware.title")}</h3>
            {hardware?.hasInitial && (
              <Button
                size="sm"
                variant="ghost"
                disabled={busy}
                onClick={() => {
                  setBusy(true);
                  ipc
                    .hardwareRestoreInitial(display.id)
                    .then(onRestored)
                    .catch((reason: unknown) => setError(errorCode(reason)))
                    .finally(() => setBusy(false));
                }}
              >
                <RotateCcw aria-hidden />
                {t("monitor.hardware.restore")}
              </Button>
            )}
          </div>
          {!hardware && loading ? (
            <p className="text-sm text-muted">{t("monitor.reading")}</p>
          ) : hasHardware ? (
            <div className="grid gap-5 md:grid-cols-2">
              {hardware?.panelBrightness != null && (
                <ParamSlider
                  id={`panel-${display.id}`}
                  label={t("monitor.hardware.panelBrightness")}
                  value={draft("panel", hardware.panelBrightness)}
                  range={{ min: 0, max: 100, default: hardware.panelBrightness, step: 1 }}
                  display={t("monitor.units.percent", {
                    value: draft("panel", hardware.panelBrightness),
                  })}
                  disabled={busy}
                  resetTitle={t("monitor.hardware.dragHint")}
                  onChange={(value) => setDraft("panel", value)}
                  onCommit={writePanel}
                />
              )}
              {controls.map((control) => {
                const value = hardware?.ddc[control];
                if (!value) return null;
                return (
                  <ParamSlider
                    key={control}
                    id={`ddc-${display.id}-${control}`}
                    label={t(`monitor.hardware.${control}`)}
                    value={draft(control, value.current)}
                    range={{ min: 0, max: value.max, default: value.current, step: 1 }}
                    display={`${draft(control, value.current)} / ${value.max}`}
                    disabled={busy}
                    resetTitle={t("monitor.hardware.dragHint")}
                    onChange={(next) => setDraft(control, next)}
                    onCommit={(next) => writeDdc(control, next)}
                  />
                );
              })}
            </div>
          ) : (
            <p className="text-sm text-muted">
              {display.internal ? t("monitor.hardware.noPanel") : t("monitor.hardware.noDdc")}
            </p>
          )}
          {error && (
            <p role="alert" className="text-xs text-warning">
              {t(`errors.${error}`)}
            </p>
          )}
        </section>
        <TrimSection display={display} />
      </CardContent>
    </Card>
  );
}

/** Gamma, calidez y ganancia RGB propias de la pantalla (por rampa, sobre el look). */
function TrimSection({ display }: { display: DisplayInfo }) {
  const { t } = useTranslation();
  const requested = useEngineStore((store) => store.requested);
  const setRequested = useEngineStore((store) => store.setRequested);
  const saved = requested?.trims[display.id] ?? NEUTRAL_TRIM;
  const [draft, setDraft] = useState<DisplayTrim | null>(null);
  const trim = draft ?? saved;

  const send = (next: DisplayTrim, preview: boolean) => {
    if (preview) setDraft(next);
    ipc
      .displaysSetTrim(display.id, next, preview)
      .then((result) => {
        if (!preview) {
          setRequested(result);
          setDraft(null);
        }
      })
      .catch(() => setDraft(null));
  };
  const channels = ["red", "green", "blue"] as const;
  const changed = JSON.stringify(saved) !== JSON.stringify(NEUTRAL_TRIM);

  return (
    <section className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex flex-col gap-1">
          <h3 className="text-sm font-medium text-fg">{t("monitor.trim.title")}</h3>
          <p className="text-xs text-muted">{t("monitor.trim.description")}</p>
        </div>
        {changed && (
          <Button
            size="sm"
            variant="ghost"
            onClick={() => {
              ipc
                .displaysSetTrim(display.id, null, false)
                .then(setRequested)
                .catch(() => undefined);
            }}
          >
            <RotateCcw aria-hidden />
            {t("monitor.trim.reset")}
          </Button>
        )}
      </div>
      {requested && !requested.colorEnabled && (
        <p className="text-xs text-warning">{t("monitor.trim.colorOff")}</p>
      )}
      <div className="grid gap-5 md:grid-cols-2">
        <ParamSlider
          id={`trim-gamma-${display.id}`}
          label={t("monitor.trim.gamma")}
          value={trim.gamma}
          range={ranges.trimGamma}
          display={trim.gamma.toFixed(2)}
          resetTitle={t("display.resetParam")}
          onChange={(gamma) => send({ ...trim, gamma }, true)}
          onCommit={(gamma) => send({ ...trim, gamma }, false)}
        />
        <ParamSlider
          id={`trim-warmth-${display.id}`}
          label={t("monitor.trim.warmth")}
          value={trim.warmth}
          range={ranges.temperature}
          display={String(Math.round(trim.warmth))}
          scale={[t("monitor.trim.cool"), t("monitor.trim.warm")]}
          resetTitle={t("display.resetParam")}
          onChange={(warmth) => send({ ...trim, warmth }, true)}
          onCommit={(warmth) => send({ ...trim, warmth }, false)}
        />
        {channels.map((channel, index) => {
          const withGain = (value: number): DisplayTrim => {
            const gain: [number, number, number] = [...trim.gain];
            gain[index] = value;
            return { ...trim, gain };
          };
          return (
            <ParamSlider
              key={channel}
              id={`trim-${channel}-${display.id}`}
              label={t(`monitor.trim.${channel}`)}
              value={trim.gain[index]}
              range={ranges.gain}
              display={trim.gain[index].toFixed(2)}
              resetTitle={t("display.resetParam")}
              onChange={(value) => send(withGain(value), true)}
              onCommit={(value) => send(withGain(value), false)}
            />
          );
        })}
      </div>
    </section>
  );
}
