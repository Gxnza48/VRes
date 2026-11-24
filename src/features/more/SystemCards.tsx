import { ClipboardCopy, FileText, LogOut, RotateCcw, Stethoscope } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { ipc } from "@/lib/ipc";
import { saveSettings, useSettingsStore } from "@/state/settings";

import notices from "../../../THIRD-PARTY-NOTICES.md?raw";

/** Qué pasa al cerrar la ventana. */
export function BehaviorCard() {
  const { t } = useTranslation();
  const settings = useSettingsStore((store) => store.settings);
  if (!settings) return null;
  const closeToTray = settings.behavior.closeToTray;

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <LogOut aria-hidden className="size-4 text-accent" />
          {t("more.behavior.title")}
        </CardTitle>
        <CardDescription>{t("more.behavior.description")}</CardDescription>
      </CardHeader>
      <CardContent className="gap-2">
        <label className="flex items-center gap-2 text-sm text-fg">
          <input
            type="checkbox"
            checked={closeToTray}
            onChange={(event) =>
              void saveSettings({ ...settings, behavior: { closeToTray: event.target.checked } })
            }
            className="size-4 accent-accent"
          />
          {t("more.behavior.closeToTray")}
        </label>
        {!closeToTray && <p className="text-xs text-muted">{t("more.behavior.closeQuits")}</p>}
      </CardContent>
    </Card>
  );
}

/** Reporte de diagnóstico: se ve antes de copiarlo y nunca se envía solo. */
export function DiagnosticsCard() {
  const { t } = useTranslation();
  const [report, setReport] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [failed, setFailed] = useState(false);

  const load = () =>
    ipc
      .diagReport()
      .then((text) => {
        setReport(text);
        setFailed(false);
        return text;
      })
      .catch(() => {
        setFailed(true);
        return null;
      });

  const copy = () => {
    void (report ? Promise.resolve(report) : load()).then((text) => {
      if (!text) return;
      navigator.clipboard
        .writeText(text)
        .then(() => {
          setCopied(true);
          setTimeout(() => setCopied(false), 2000);
        })
        .catch(() => setFailed(true));
    });
  };

  return (
    <Card className="lg:col-span-2">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Stethoscope aria-hidden className="size-4 text-accent" />
          {t("more.diagnostics.title")}
        </CardTitle>
        <CardDescription>{t("more.diagnostics.description")}</CardDescription>
      </CardHeader>
      <CardContent className="gap-3">
        <div className="flex flex-wrap gap-2">
          <Button variant="secondary" size="sm" onClick={() => void load()}>
            <FileText aria-hidden />
            {t("more.diagnostics.show")}
          </Button>
          <Button variant="secondary" size="sm" onClick={copy}>
            <ClipboardCopy aria-hidden />
            {copied ? t("more.diagnostics.copied") : t("more.diagnostics.copy")}
          </Button>
        </div>
        {failed && (
          <p role="alert" className="text-xs text-warning">
            {t("more.diagnostics.failed")}
          </p>
        )}
        {report && (
          <pre
            aria-label={t("more.diagnostics.title")}
            className="max-h-72 overflow-auto rounded-lg border border-border bg-surface-2 p-3 text-xs text-muted select-text"
          >
            {report}
          </pre>
        )}
      </CardContent>
    </Card>
  );
}

/** Licencia de VRes y de lo que usa. */
export function LicensesCard() {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  return (
    <Card className="lg:col-span-2">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <FileText aria-hidden className="size-4 text-accent" />
          {t("more.licenses.title")}
        </CardTitle>
        <CardDescription>{t("more.licenses.description")}</CardDescription>
      </CardHeader>
      <CardContent className="gap-3">
        <Button variant="ghost" size="sm" aria-expanded={open} onClick={() => setOpen(!open)}>
          {open ? t("more.licenses.hide") : t("more.licenses.show")}
        </Button>
        {open && (
          <pre className="max-h-72 overflow-auto whitespace-pre-wrap rounded-lg border border-border bg-surface-2 p-3 text-xs text-muted select-text">
            {notices}
          </pre>
        )}
      </CardContent>
    </Card>
  );
}

/** Vuelve todo a como estaba al instalar, con confirmación. */
export function ResetCard() {
  const { t } = useTranslation();
  const setSettings = useSettingsStore((store) => store.setSettings);
  const [confirming, setConfirming] = useState(false);
  const [state, setState] = useState<"idle" | "working" | "done" | "failed">("idle");

  const reset = () => {
    setState("working");
    ipc
      .settingsReset()
      .then((settings) => {
        setSettings(settings);
        setState("done");
        setConfirming(false);
      })
      .catch(() => setState("failed"));
  };

  return (
    <Card className="lg:col-span-2">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <RotateCcw aria-hidden className="size-4 text-danger" />
          {t("more.reset.title")}
        </CardTitle>
        <CardDescription>{t("more.reset.description")}</CardDescription>
      </CardHeader>
      <CardContent className="gap-3">
        {confirming ? (
          <div className="flex flex-col gap-3 rounded-xl border border-danger/40 bg-danger/10 p-3">
            <p className="text-sm text-fg">{t("more.reset.confirm")}</p>
            <div className="flex flex-wrap gap-2">
              <Button variant="danger" size="sm" disabled={state === "working"} onClick={reset}>
                {t("more.reset.yes")}
              </Button>
              <Button variant="ghost" size="sm" onClick={() => setConfirming(false)}>
                {t("more.reset.cancel")}
              </Button>
            </div>
          </div>
        ) : (
          <div>
            <Button variant="secondary" size="sm" onClick={() => setConfirming(true)}>
              <RotateCcw aria-hidden />
              {t("more.reset.button")}
            </Button>
          </div>
        )}
        {state === "done" && <p className="text-xs text-success">{t("more.reset.done")}</p>}
        {state === "failed" && (
          <p role="alert" className="text-xs text-warning">
            {t("more.reset.failed")}
          </p>
        )}
      </CardContent>
    </Card>
  );
}
