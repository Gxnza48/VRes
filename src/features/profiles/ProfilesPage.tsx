import { Gamepad2, Plus, RefreshCw, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { PageHeader } from "@/components/common/PageHeader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { CrosshairChoice } from "@/lib/bindings/CrosshairChoice";
import type { GameProfile } from "@/lib/bindings/GameProfile";
import type { Look } from "@/lib/bindings/Look";
import type { ProfileSettings } from "@/lib/bindings/ProfileSettings";
import type { RunningApp } from "@/lib/bindings/RunningApp";
import { ipc } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { sizeLabel } from "@/features/resolution/format";
import { useCatalogStore } from "@/state/catalog";
import { useEngineStore } from "@/state/engine";
import { saveSettings, useSettingsStore } from "@/state/settings";

const field = "rounded-lg border border-border bg-surface-2 px-2 py-1.5 text-sm text-fg";
const CHOICES: CrosshairChoice[] = ["keep", "on", "off"];

interface GameRow {
  id: string;
  name: string;
  executables: string[];
  antiCheat: string | null;
  custom: boolean;
}

function sameLook(a: Look | null, b: Look | null): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/** Pestaña Perfiles: qué aplica VRes en cada juego. */
export function ProfilesPage() {
  const { t } = useTranslation();
  const settings = useSettingsStore((store) => store.settings);
  const displays = useEngineStore((store) => store.displays);
  const activeProfile = useEngineStore((store) => store.activeProfile);
  const { lookPresets, games, load } = useCatalogStore();
  const [candidates, setCandidates] = useState<RunningApp[]>([]);
  const [picked, setPicked] = useState("");

  useEffect(() => load(), [load]);

  const refresh = () => {
    ipc
      .gamesRunningCandidates()
      .then(setCandidates)
      .catch(() => setCandidates([]));
  };
  useEffect(refresh, []);

  if (!settings) {
    return <PageHeader title={t("profiles.title")} subtitle={t("profiles.subtitle")} />;
  }
  const config = settings.profiles;
  const save = (next: ProfileSettings) => void saveSettings({ ...settings, profiles: next });

  const rows: GameRow[] = [
    ...games.map((game) => ({
      id: game.id,
      name: game.name,
      executables: game.executables,
      antiCheat: game.antiCheat,
      custom: false,
    })),
    ...config.customGames.map((game) => ({
      id: game.id,
      name: game.name,
      executables: [game.executable],
      antiCheat: null,
      custom: true,
    })),
  ];
  const gameName = (id: string | null) => rows.find((row) => row.id === id)?.name ?? id;

  const profileOf = (gameId: string): GameProfile =>
    config.profiles.find((profile) => profile.gameId === gameId) ?? {
      gameId,
      enabled: false,
      look: null,
      crosshair: "keep",
      resolution: null,
    };
  const update = (gameId: string, patch: Partial<GameProfile>) => {
    const next = { ...profileOf(gameId), ...patch };
    save({
      ...config,
      profiles: [...config.profiles.filter((profile) => profile.gameId !== gameId), next],
    });
  };

  // Opciones de look: el del escritorio, los incluidos (primero los del juego) y los tuyos.
  const lookOptions = (gameId: string) => [
    ...lookPresets
      .filter((preset) => preset.game === gameId || preset.game === null)
      .sort((a, b) => Number(b.game === gameId) - Number(a.game === gameId))
      .map((preset) => ({ key: `preset:${preset.id}`, label: t(`looks.presets.${preset.id}`), look: preset.look, group: "included" })),
    ...settings.library.looks.map((saved) => ({ key: `saved:${saved.id}`, label: saved.name, look: saved.look, group: "mine" })),
  ];

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        title={t("profiles.title")}
        subtitle={t("profiles.subtitle")}
        aside={
          <Button
            variant={config.autoSwitch ? "primary" : "secondary"}
            onClick={() => save({ ...config, autoSwitch: !config.autoSwitch })}
          >
            <Gamepad2 aria-hidden />
            {t("profiles.autoSwitch")}: {config.autoSwitch ? t("profiles.on") : t("profiles.off")}
          </Button>
        }
      />

      <p className={cn("text-sm", activeProfile ? "text-accent" : "text-muted")} role="status">
        {activeProfile
          ? t("profiles.active", { game: gameName(activeProfile) })
          : t("profiles.inactive")}
      </p>

      {rows.map((row) => {
        const profile = profileOf(row.id);
        const options = lookOptions(row.id);
        const selected =
          profile.look === null
            ? "desktop"
            : (options.find((option) => sameLook(option.look, profile.look))?.key ?? "custom");
        return (
          <Card key={row.id} className={cn(profile.enabled && "border-accent/50")}>
            <CardHeader className="flex-row items-start justify-between gap-4">
              <div className="flex flex-col gap-1.5">
                <CardTitle>{row.name}</CardTitle>
                <CardDescription>
                  {row.executables.join(" · ")}
                  {row.antiCheat ? ` · ${t("profiles.antiCheat", { name: row.antiCheat })}` : ""}
                </CardDescription>
              </div>
              <div className="flex items-center gap-2">
                {activeProfile === row.id && <Badge tone="accent">{t("profiles.inUse")}</Badge>}
                <label className="flex items-center gap-2 text-sm text-fg">
                  <input
                    type="checkbox"
                    checked={profile.enabled}
                    onChange={(event) => update(row.id, { enabled: event.target.checked })}
                    className="size-4 accent-accent"
                  />
                  {t("profiles.profileEnabled")}
                </label>
                {row.custom && (
                  <Button
                    size="sm"
                    variant="ghost"
                    aria-label={t("profiles.custom.remove")}
                    onClick={() =>
                      save({
                        ...config,
                        customGames: config.customGames.filter((game) => game.id !== row.id),
                        profiles: config.profiles.filter((item) => item.gameId !== row.id),
                      })
                    }
                  >
                    <Trash2 aria-hidden />
                  </Button>
                )}
              </div>
            </CardHeader>
            {profile.enabled && (
              <CardContent className="grid gap-4 md:grid-cols-3">
                <label className="flex flex-col gap-1 text-sm text-muted">
                  {t("profiles.look")}
                  <select
                    value={selected}
                    onChange={(event) => {
                      const key = event.target.value;
                      if (key === "custom") return;
                      const option = options.find((item) => item.key === key);
                      update(row.id, { look: key === "desktop" ? null : (option?.look ?? null) });
                    }}
                    className={field}
                  >
                    <option value="desktop">{t("profiles.lookDesktop")}</option>
                    {selected === "custom" && <option value="custom">{t("profiles.lookCustom")}</option>}
                    <optgroup label={t("profiles.includedGroup")}>
                      {options
                        .filter((option) => option.group === "included")
                        .map((option) => (
                          <option key={option.key} value={option.key}>
                            {option.label}
                          </option>
                        ))}
                    </optgroup>
                    {options.some((option) => option.group === "mine") && (
                      <optgroup label={t("profiles.mineGroup")}>
                        {options
                          .filter((option) => option.group === "mine")
                          .map((option) => (
                            <option key={option.key} value={option.key}>
                              {option.label}
                            </option>
                          ))}
                      </optgroup>
                    )}
                  </select>
                </label>
                <label className="flex flex-col gap-1 text-sm text-muted">
                  {t("profiles.crosshair")}
                  <select
                    value={profile.crosshair}
                    onChange={(event) => update(row.id, { crosshair: event.target.value as CrosshairChoice })}
                    className={field}
                  >
                    {CHOICES.map((choice) => (
                      <option key={choice} value={choice}>
                        {t(`profiles.crosshairChoices.${choice}`)}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="flex flex-col gap-1 text-sm text-muted">
                  {t("profiles.resolution")}
                  <select
                    value={
                      profile.resolution
                        ? settings.resolution.confirmed.findIndex(
                            (item) => JSON.stringify(item) === JSON.stringify(profile.resolution),
                          )
                        : -1
                    }
                    onChange={(event) => {
                      const index = Number(event.target.value);
                      update(row.id, { resolution: settings.resolution.confirmed[index] ?? null });
                    }}
                    className={field}
                  >
                    <option value={-1}>{t("profiles.resolutionNone")}</option>
                    {settings.resolution.confirmed.map((item, index) => {
                      const display = displays.find((candidate) => candidate.id === item.displayId);
                      return (
                        <option key={index} value={index}>
                          {sizeLabel(item.mode.width, item.mode.height)} · {item.mode.refreshHz} Hz ·{" "}
                          {display?.friendlyName || display?.gdiName || item.displayId}
                        </option>
                      );
                    })}
                  </select>
                  <span className="text-xs text-muted">{t("profiles.resolutionHint")}</span>
                </label>
              </CardContent>
            )}
          </Card>
        );
      })}

      <Card>
        <CardHeader>
          <CardTitle>{t("profiles.custom.title")}</CardTitle>
          <CardDescription>{t("profiles.custom.description")}</CardDescription>
        </CardHeader>
        <CardContent className="flex-row flex-wrap items-center gap-2">
          <select
            value={picked}
            aria-label={t("profiles.custom.pick")}
            onChange={(event) => setPicked(event.target.value)}
            className={cn(field, "min-w-64")}
          >
            <option value="">
              {candidates.length === 0 ? t("profiles.custom.empty") : t("profiles.custom.pick")}
            </option>
            {candidates.map((app) => (
              <option key={app.executable} value={app.executable}>
                {app.title} ({app.executable})
              </option>
            ))}
          </select>
          <Button size="sm" variant="ghost" onClick={refresh}>
            <RefreshCw aria-hidden />
            {t("profiles.custom.refresh")}
          </Button>
          <Button
            variant="primary"
            disabled={!picked}
            onClick={() => {
              const app = candidates.find((item) => item.executable === picked);
              if (!app) return;
              const id = `custom:${app.executable.toLowerCase()}`;
              save({
                ...config,
                customGames: [
                  ...config.customGames.filter((game) => game.id !== id),
                  { id, name: app.title.slice(0, 60) || app.executable, executable: app.executable },
                ],
              });
              setPicked("");
            }}
          >
            <Plus aria-hidden />
            {t("profiles.custom.add")}
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
