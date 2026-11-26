import {
  ArrowRight,
  Contrast,
  Copy,
  Crosshair as CrosshairIcon,
  ExternalLink,
  Gamepad2,
  Keyboard,
  Repeat,
  Scaling,
} from "lucide-react";
import { type ReactNode, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { TabId } from "@/app/nav";
import { HotkeyRecorder } from "@/components/common/HotkeyRecorder";
import { PageHeader } from "@/components/common/PageHeader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { ErrorCode } from "@/lib/bindings/ErrorCode";
import type { Hotkey } from "@/lib/bindings/Hotkey";
import type { Settings } from "@/lib/bindings/Settings";
import { ipc } from "@/lib/ipc";
import { CROSSHAIRS_URL, openSite } from "@/lib/links";
import { cn } from "@/lib/utils";
import { CrosshairCanvas } from "@/features/crosshair/CrosshairCanvas";
import { setColorEnabled } from "@/features/display/actions";
import { sizeLabel } from "@/features/resolution/format";
import { useAppStore } from "@/state/app";
import { useCatalogStore } from "@/state/catalog";
import { useEngineStore } from "@/state/engine";
import { saveSettings, useSettingsStore } from "@/state/settings";

import { SetupPaste } from "./SetupPaste";

/** Tarjeta de un bloque de la config principal, con su botón para editarlo. */
function SummaryCard({
  icon,
  title,
  badge,
  tab,
  children,
}: {
  icon: ReactNode;
  title: string;
  badge: ReactNode;
  tab: TabId;
  children: ReactNode;
}) {
  const { t } = useTranslation();
  const setTab = useAppStore((state) => state.setTab);
  return (
    <Card className="flex flex-col">
      <CardHeader className="flex-row items-center justify-between gap-3">
        <CardTitle className="flex items-center gap-2">
          {icon}
          {title}
        </CardTitle>
        {badge}
      </CardHeader>
      <CardContent className="flex flex-1 flex-col gap-4">
        {children}
        <Button size="sm" variant="ghost" className="mt-auto self-start" onClick={() => setTab(tab)}>
          {t("home.edit")}
          <ArrowRight aria-hidden />
        </Button>
      </CardContent>
    </Card>
  );
}

/** Pestaña Inicio: la config principal (colores, mira, resolución y atajos) en un lugar. */
export function HomePage() {
  const { t } = useTranslation();
  const settings = useSettingsStore((store) => store.settings);
  const requested = useEngineStore((store) => store.requested);
  const displays = useEngineStore((store) => store.displays);
  const activeProfile = useEngineStore((store) => store.activeProfile);
  const setTab = useAppStore((state) => state.setTab);
  const { games, load } = useCatalogStore();
  const [error, setError] = useState<ErrorCode | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => load(), [load]);

  if (!settings || !requested) {
    return <PageHeader title={t("home.title")} subtitle={t("home.subtitle")} />;
  }

  const save = (next: Settings) => {
    setError(null);
    void saveSettings(next).then((code) => {
      if (code) setError(code);
    });
  };
  const profiles = settings.profiles;
  const look = requested.look;
  const crosshair = settings.crosshair;
  const toggle = settings.resolution.toggle;
  const toggleDisplay = toggle ? displays.find((display) => display.id === toggle.displayId) : null;
  const hotkeys = settings.hotkeys;
  const allGames = [
    ...games.map((game) => ({ id: game.id, name: game.name })),
    ...profiles.customGames.map((game) => ({ id: game.id, name: game.name })),
  ];
  const gameName = (id: string) => allGames.find((game) => game.id === id)?.name ?? id;
  const withProfile = profiles.profiles.filter((profile) => profile.enabled);

  const setHotkey = (key: "crosshairToggle" | "resolutionToggle", hotkey: Hotkey | null) =>
    save({ ...settings, hotkeys: { ...hotkeys, [key]: hotkey } });

  const toggleGame = (id: string) => {
    const current = profiles.mainGames;
    const mainGames = current.includes(id)
      ? current.filter((game) => game !== id)
      : [...current, id];
    save({ ...settings, profiles: { ...profiles, mainGames } });
  };

  const copySetup = async () => {
    const lines = [
      await ipc.shareEncodeLook(look),
      await ipc.shareEncodeCrosshair(crosshair.current),
    ];
    if (toggle) lines.push(await ipc.shareEncodeResolution(toggle.mode));
    await navigator.clipboard.writeText(lines.join("\n"));
    setCopied(true);
    window.setTimeout(() => setCopied(false), 2000);
  };

  const onOff = (on: boolean) => (
    <Badge tone={on ? "accent" : undefined}>{on ? t("home.on") : t("home.off")}</Badge>
  );

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        title={t("home.title")}
        subtitle={t("home.subtitle")}
        aside={
          <Button onClick={() => void copySetup().catch(() => undefined)}>
            <Copy aria-hidden />
            {copied ? t("home.copied") : t("home.copySetup")}
          </Button>
        }
      />
      {error && (
        <p role="alert" className="text-sm text-danger">
          {t(`errors.${error}`)}
        </p>
      )}

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Gamepad2 aria-hidden className="size-4 text-accent" />
            {t("home.scope.title")}
          </CardTitle>
          <CardDescription>{t("home.scope.description")}</CardDescription>
        </CardHeader>
        <CardContent className="gap-4">
          <div role="radiogroup" aria-label={t("home.scope.title")} className="grid gap-3 md:grid-cols-2">
            {([false, true] as const).map((only) => {
              const selected = profiles.mainOnlyInGames === only;
              return (
                <button
                  key={String(only)}
                  type="button"
                  role="radio"
                  aria-checked={selected}
                  onClick={() => save({ ...settings, profiles: { ...profiles, mainOnlyInGames: only } })}
                  className={cn(
                    "flex flex-col gap-1 rounded-xl border p-4 text-left transition-colors",
                    selected
                      ? "border-accent bg-accent/10"
                      : "border-border bg-surface-2 hover:border-muted",
                  )}
                >
                  <span className="text-sm font-semibold text-fg">
                    {t(only ? "home.scope.games" : "home.scope.always")}
                  </span>
                  <span className="text-xs text-muted">
                    {t(only ? "home.scope.gamesHint" : "home.scope.alwaysHint")}
                  </span>
                </button>
              );
            })}
          </div>
          {profiles.mainOnlyInGames && (
            <div className="flex flex-col gap-2">
              <p className="text-xs text-muted">{t("home.scope.pick")}</p>
              <div className="flex flex-wrap gap-2">
                {allGames.map((game) => {
                  const on = profiles.mainGames.includes(game.id);
                  return (
                    <button
                      key={game.id}
                      type="button"
                      aria-pressed={on}
                      onClick={() => toggleGame(game.id)}
                      className={cn(
                        "rounded-full border px-3 py-1 text-xs transition-colors",
                        on
                          ? "border-accent bg-accent font-semibold text-accent-fg"
                          : "border-border bg-surface-2 text-muted hover:text-fg",
                      )}
                    >
                      {game.name}
                    </button>
                  );
                })}
              </div>
              <p className="text-xs text-muted">
                {profiles.mainGames.length === 0 ? t("home.scope.anyGame") : null}
              </p>
            </div>
          )}
          <p className="text-xs text-muted">{t("home.scope.fullscreen")}</p>
        </CardContent>
      </Card>

      <div className="grid gap-6 md:grid-cols-3">
        <SummaryCard
          icon={<Contrast aria-hidden className="size-4 text-accent" />}
          title={t("home.colors.title")}
          badge={onOff(requested.colorEnabled)}
          tab="display"
        >
          <dl className="grid grid-cols-2 gap-2 text-sm">
            <dt className="text-muted">{t("home.colors.saturation")}</dt>
            <dd className="text-right tabular-nums text-fg">{Math.round(look.saturation)} %</dd>
            <dt className="text-muted">{t("home.colors.vibrance")}</dt>
            <dd className="text-right tabular-nums text-fg">{Math.round(look.vibrance)}</dd>
          </dl>
          <label className="flex items-center gap-2 text-sm text-fg">
            <input
              type="checkbox"
              checked={requested.colorEnabled}
              onChange={(event) => setColorEnabled(event.target.checked)}
            />
            {t("home.colors.enabled")}
          </label>
        </SummaryCard>

        <SummaryCard
          icon={<CrosshairIcon aria-hidden className="size-4 text-accent" />}
          title={t("home.crosshair.title")}
          badge={onOff(crosshair.enabled)}
          tab="crosshair"
        >
          <div className="flex items-center gap-4">
            <CrosshairCanvas crosshair={crosshair.current} box={72} zoom={2} />
            <label className="flex items-center gap-2 text-sm text-fg">
              <input
                type="checkbox"
                checked={crosshair.enabled}
                onChange={(event) =>
                  save({ ...settings, crosshair: { ...crosshair, enabled: event.target.checked } })
                }
              />
              {t("home.crosshair.enabled")}
            </label>
          </div>
          <Button size="sm" className="self-start" onClick={() => openSite(CROSSHAIRS_URL)}>
            <ExternalLink aria-hidden />
            {t("home.crosshair.browse")}
          </Button>
        </SummaryCard>

        <SummaryCard
          icon={<Scaling aria-hidden className="size-4 text-accent" />}
          title={t("home.resolution.title")}
          badge={toggle ? <Badge tone="accent">{sizeLabel(toggle.mode.width, toggle.mode.height)}</Badge> : <Badge>—</Badge>}
          tab="resolution"
        >
          {toggle ? (
            <>
              <p className="text-sm text-fg">
                {t("home.resolution.target", {
                  mode: `${sizeLabel(toggle.mode.width, toggle.mode.height)} · ${t(`resolution.scaling.${toggle.mode.scaling}`)}`,
                  display: toggleDisplay?.friendlyName || toggleDisplay?.gdiName || "—",
                })}
              </p>
              <Button size="sm" className="self-start" onClick={() => void ipc.resolutionToggle()}>
                <Repeat aria-hidden />
                {t("home.resolution.toggle")}
              </Button>
            </>
          ) : (
            <p className="text-sm text-muted">{t("home.resolution.none")}</p>
          )}
        </SummaryCard>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Keyboard aria-hidden className="size-4 text-accent" />
            {t("home.hotkeys.title")}
          </CardTitle>
          <CardDescription>{t("home.hotkeys.description")}</CardDescription>
        </CardHeader>
        <CardContent className="gap-4">
          <div className="grid items-center gap-x-6 gap-y-4 md:grid-cols-[12rem_1fr]">
            <span className="text-sm text-muted">{t("home.hotkeys.crosshair")}</span>
            <HotkeyRecorder
              value={hotkeys.crosshairToggle}
              others={hotkeys.resolutionToggle ? [hotkeys.resolutionToggle] : []}
              onChange={(hotkey) => setHotkey("crosshairToggle", hotkey)}
            />
            <span className="text-sm text-muted">{t("home.hotkeys.resolution")}</span>
            <HotkeyRecorder
              value={hotkeys.resolutionToggle}
              others={hotkeys.crosshairToggle ? [hotkeys.crosshairToggle] : []}
              onChange={(hotkey) => setHotkey("resolutionToggle", hotkey)}
            />
            <span className="text-sm text-muted">{t("home.hotkeys.emergency")}</span>
            <span className="text-sm text-fg">Ctrl + Alt + Shift + R</span>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex-row items-start justify-between gap-4">
          <div className="flex flex-col gap-1">
            <CardTitle>{t("home.games.title")}</CardTitle>
            <CardDescription>{t("home.games.description")}</CardDescription>
          </div>
          <Button size="sm" onClick={() => setTab("profiles")}>
            {t("home.games.manage")}
            <ArrowRight aria-hidden />
          </Button>
        </CardHeader>
        <CardContent>
          {withProfile.length === 0 || !profiles.autoSwitch ? (
            <p className="text-sm text-muted">{t("home.games.none")}</p>
          ) : (
            <ul className="flex flex-wrap gap-2">
              {withProfile.map((profile) => (
                <li key={profile.gameId}>
                  <Badge tone={activeProfile === profile.gameId ? "accent" : undefined}>
                    {gameName(profile.gameId)}
                    {activeProfile === profile.gameId ? ` · ${t("home.games.active")}` : ""}
                  </Badge>
                </li>
              ))}
            </ul>
          )}
        </CardContent>
      </Card>

      <SetupPaste />
    </div>
  );
}
