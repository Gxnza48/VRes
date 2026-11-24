import {
  ClipboardCopy,
  Download,
  ExternalLink,
  Languages,
  Power,
  RefreshCw,
  ShieldCheck,
} from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { PageHeader } from "@/components/common/PageHeader";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { setLanguage } from "@/i18n";
import { LANGUAGES, type Lang, toSupported } from "@/i18n/language";
import type { UpdateInfo } from "@/lib/bindings/UpdateInfo";
import { events } from "@/lib/events";
import { inTauri, ipc, isIpcError } from "@/lib/ipc";
import { ISSUES_URL, REPOSITORY_URL } from "@/lib/links";
import { useAppStore } from "@/state/app";
import { useEngineStore } from "@/state/engine";

type UpdateState =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "upToDate" }
  | { kind: "available"; info: UpdateInfo }
  | { kind: "installing"; percent: number }
  | { kind: "error"; message: string };

function describe(error: unknown): string {
  return isIpcError(error) ? error.message : String(error);
}

/** Sección "Más": idioma, actualizaciones, seguridad y acerca de. */
export function MorePage() {
  const { t } = useTranslation();

  return (
    <>
      <PageHeader title={t("more.title")} subtitle={t("more.subtitle")} />
      <div className="grid gap-4 lg:grid-cols-2">
        <LanguageCard />
        <StartupCard />
        <UpdatesCard />
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <ShieldCheck aria-hidden className="size-4 text-success" />
              {t("more.safety.title")}
            </CardTitle>
            <CardDescription>{t("more.safety.description")}</CardDescription>
          </CardHeader>
        </Card>
        <AboutCard />
      </div>
    </>
  );
}

function LanguageCard() {
  const { t, i18n } = useTranslation();
  const current = toSupported(i18n.resolvedLanguage) ?? "es";

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Languages aria-hidden className="size-4 text-accent" />
          {t("more.language.title")}
        </CardTitle>
        <CardDescription>{t("more.language.description")}</CardDescription>
      </CardHeader>
      <CardContent>
        <select
          aria-label={t("more.language.title")}
          value={current}
          onChange={(event) => void setLanguage(event.target.value as Lang)}
          className="h-9 w-full max-w-xs rounded-lg border border-border bg-surface-2 px-3 text-sm text-fg"
        >
          {LANGUAGES.map((lang) => (
            <option key={lang} value={lang}>
              {t(`more.languages.${lang}`)}
            </option>
          ))}
        </select>
      </CardContent>
    </Card>
  );
}

function StartupCard() {
  const { t } = useTranslation();
  const tauri = inTauri();
  const [enabled, setEnabled] = useState<boolean | null>(null);

  useEffect(() => {
    if (!tauri) return;
    ipc
      .settingsGetAutostart()
      .then(setEnabled)
      .catch(() => setEnabled(false));
  }, [tauri]);

  const toggle = () => {
    const next = !enabled;
    setEnabled(next);
    ipc
      .settingsSetAutostart(next)
      .then(setEnabled)
      .catch(() => setEnabled(!next));
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Power aria-hidden className="size-4 text-accent" />
          {t("more.startup.title")}
        </CardTitle>
        <CardDescription>{t("more.startup.description")}</CardDescription>
      </CardHeader>
      <CardContent>
        <label className="flex cursor-pointer items-center gap-3 text-sm text-fg">
          <input
            type="checkbox"
            checked={enabled ?? false}
            disabled={!tauri || enabled === null}
            onChange={toggle}
            className="size-4 cursor-pointer accent-accent disabled:cursor-not-allowed"
          />
          {t("more.startup.label")}
        </label>
      </CardContent>
    </Card>
  );
}

function UpdatesCard() {
  const { t } = useTranslation();
  const [state, setState] = useState<UpdateState>({ kind: "idle" });
  const tauri = inTauri();

  async function check() {
    setState({ kind: "checking" });
    try {
      const info = await ipc.updateCheck();
      setState(info ? { kind: "available", info } : { kind: "upToDate" });
    } catch (error) {
      setState({ kind: "error", message: describe(error) });
    }
  }

  async function install() {
    setState({ kind: "installing", percent: 0 });
    const unlisten = await events.onUpdateProgress(({ downloaded, total }) => {
      const percent = total ? Math.min(100, Math.round((downloaded / total) * 100)) : 0;
      setState({ kind: "installing", percent });
    });
    try {
      // En Windows, el instalador cierra VRes y lo vuelve a abrir.
      await ipc.updateInstall();
    } catch (error) {
      setState({ kind: "error", message: describe(error) });
    } finally {
      unlisten();
    }
  }

  const busy = state.kind === "checking" || state.kind === "installing";

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <RefreshCw aria-hidden className="size-4 text-accent" />
          {t("more.updates.title")}
        </CardTitle>
        <CardDescription>{t("more.updates.description")}</CardDescription>
      </CardHeader>
      <CardContent>
        {!tauri ? (
          <p className="text-sm text-muted">{t("more.updates.onlyInApp")}</p>
        ) : (
          <>
            <div className="flex flex-wrap items-center gap-2">
              <Button onClick={() => void check()} disabled={busy}>
                <RefreshCw aria-hidden />
                {state.kind === "checking" ? t("more.updates.checking") : t("more.updates.check")}
              </Button>
              {state.kind === "available" && (
                <Button variant="primary" onClick={() => void install()}>
                  <Download aria-hidden />
                  {t("more.updates.install")}
                </Button>
              )}
            </div>
            <p role="status" className="min-h-5 text-sm text-muted">
              {state.kind === "upToDate" && t("more.updates.upToDate")}
              {state.kind === "available" &&
                t("more.updates.available", { version: state.info.version })}
              {state.kind === "installing" &&
                t("more.updates.installing", { percent: state.percent })}
              {state.kind === "error" && t("more.updates.error", { message: state.message })}
            </p>
          </>
        )}
      </CardContent>
    </Card>
  );
}

function AboutCard() {
  const { t } = useTranslation();
  const version = useAppStore((state) => state.bootstrap?.appVersion);
  const tauri = inTauri();
  const [copied, setCopied] = useState(false);

  /** Copia un resumen técnico (sin datos personales) para los reportes. */
  const copyDiagnostics = () => {
    const engine = useEngineStore.getState();
    const report = {
      version,
      userAgent: navigator.userAgent,
      displays: engine.displays,
      status: engine.status,
      requested: engine.requested,
    };
    navigator.clipboard
      .writeText(JSON.stringify(report, null, 2))
      .then(() => {
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      })
      .catch((error: unknown) => console.error("diagnóstico", error));
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("more.about.title")}</CardTitle>
        <CardDescription>
          {version ? `${t("more.about.version", { version })} · ` : ""}
          {t("more.about.license")}
        </CardDescription>
      </CardHeader>
      {tauri && (
        <CardContent className="flex-row flex-wrap">
          <Button variant="ghost" size="sm" onClick={() => void ipc.openExternal(REPOSITORY_URL)}>
            <ExternalLink aria-hidden />
            {t("more.about.repository")}
          </Button>
          <Button variant="ghost" size="sm" onClick={() => void ipc.openExternal(ISSUES_URL)}>
            <ExternalLink aria-hidden />
            {t("more.about.report")}
          </Button>
          <Button variant="ghost" size="sm" onClick={copyDiagnostics}>
            <ClipboardCopy aria-hidden />
            {copied ? t("more.about.copied") : t("more.about.copyDiagnostics")}
          </Button>
        </CardContent>
      )}
    </Card>
  );
}
