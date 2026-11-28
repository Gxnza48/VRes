import { ArrowRight, Gauge, RotateCcw, ShieldCheck, TriangleAlert } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { PageHeader } from "@/components/common/PageHeader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { ErrorCode } from "@/lib/bindings/ErrorCode";
import type { GameTuneStatus } from "@/lib/bindings/GameTuneStatus";
import { ipc, isIpcError } from "@/lib/ipc";
import { cn } from "@/lib/utils";

/** Valores del archivo del juego que se muestran traducidos. */
const KNOWN_VALUES = ["true", "false", "disabled", "minimum", "low", "medium", "high", "ultra", "custom"];

/** Pestaña FPS: pone los gráficos del juego en los valores que más FPS dan. */
export function FpsPage() {
  const { t } = useTranslation();
  const [games, setGames] = useState<GameTuneStatus[] | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<ErrorCode | null>(null);
  const [done, setDone] = useState<{ game: string; restore: boolean } | null>(null);

  const refresh = useCallback(() => {
    ipc
      .gameTuneStatus()
      .then(setGames)
      .catch(() => setGames([]));
  }, []);

  useEffect(refresh, [refresh]);

  const run = (game: string, restore: boolean) => {
    setBusy(game);
    setError(null);
    setDone(null);
    ipc
      .gameTuneApply(game, restore)
      .then((next) => {
        setGames((list) => list?.map((item) => (item.gameId === next.gameId ? next : item)) ?? [next]);
        setDone({ game, restore });
      })
      .catch((reason: unknown) => {
        setError(isIpcError(reason) ? reason.code : "internal");
        refresh();
      })
      .finally(() => setBusy(null));
  };

  const value = (raw: string) => {
    const plain = raw.replace(/^"|"$/g, "");
    return KNOWN_VALUES.includes(plain) ? t(`fps.values.${plain}`) : plain;
  };

  return (
    <div className="flex flex-col gap-6">
      <PageHeader title={t("fps.title")} subtitle={t("fps.subtitle")} />

      {error && (
        <p role="alert" className="text-sm text-danger">
          {t(`errors.${error}`)}
        </p>
      )}

      {games?.map((status) => {
        const pending = status.settings.filter((setting) => setting.current !== setting.target).length;
        return (
          <Card key={status.gameId}>
            <CardHeader className="flex-row items-start justify-between gap-4">
              <div className="flex flex-col gap-1">
                <CardTitle className="flex items-center gap-2">
                  <Gauge aria-hidden className="size-4 text-accent" />
                  {status.gameName}
                </CardTitle>
                <CardDescription>{t("fps.game.description")}</CardDescription>
              </div>
              {status.found && (
                <Badge tone={status.applied ? "accent" : "neutral"}>
                  {t(status.applied ? "fps.state.applied" : "fps.state.player")}
                </Badge>
              )}
            </CardHeader>
            <CardContent className="gap-4">
              {!status.found ? (
                <p className="text-sm text-muted">{t("fps.notFound")}</p>
              ) : (
                <>
                  {status.gameRunning && (
                    <p className="flex items-start gap-3 rounded-xl border border-warning/40 bg-warning/10 px-4 py-3 text-sm text-fg">
                      <TriangleAlert aria-hidden className="mt-0.5 size-4 shrink-0 text-warning" />
                      {t("fps.running")}
                    </p>
                  )}

                  <ul className="grid gap-2 md:grid-cols-2">
                    {status.settings.map((setting) => {
                      const same = setting.current === setting.target;
                      return (
                        <li
                          key={setting.key}
                          className={cn(
                            "flex items-center justify-between gap-3 rounded-xl border px-3 py-2 text-sm",
                            same ? "border-border bg-surface-2" : "border-accent/40 bg-accent/5",
                          )}
                        >
                          <span className="text-fg">{t(`fps.keys.${setting.key}`, { defaultValue: setting.key })}</span>
                          <span className="flex shrink-0 items-center gap-1.5 text-xs">
                            {same ? (
                              <span className="text-muted">{value(setting.current)}</span>
                            ) : (
                              <>
                                <span className="text-muted">{value(setting.current)}</span>
                                <ArrowRight aria-hidden className="size-3 text-accent" />
                                <span className="font-semibold text-fg">{value(setting.target)}</span>
                              </>
                            )}
                          </span>
                        </li>
                      );
                    })}
                  </ul>

                  <div className="flex flex-wrap items-center gap-3">
                    <Button
                      variant="primary"
                      size="lg"
                      disabled={busy !== null || status.gameRunning || pending === 0}
                      onClick={() => run(status.gameId, false)}
                    >
                      <Gauge aria-hidden className="size-4" />
                      {pending === 0 ? t("fps.alreadyMax") : t("fps.apply", { n: pending })}
                    </Button>
                    {status.applied && (
                      <Button
                        variant="secondary"
                        size="lg"
                        disabled={busy !== null || status.gameRunning}
                        onClick={() => run(status.gameId, true)}
                      >
                        <RotateCcw aria-hidden className="size-4" />
                        {t("fps.restore")}
                      </Button>
                    )}
                    {status.gameRunning && (
                      <Button variant="ghost" onClick={refresh}>
                        {t("fps.check")}
                      </Button>
                    )}
                  </div>

                  {busy === status.gameId && <p className="text-sm text-muted">{t("fps.applying")}</p>}
                  {done?.game === status.gameId && (
                    <p className="flex items-center gap-2 text-sm text-success">
                      <ShieldCheck aria-hidden className="size-4" />
                      {t(done.restore ? "fps.restored" : "fps.done")}
                    </p>
                  )}
                </>
              )}
            </CardContent>
          </Card>
        );
      })}

      <Card>
        <CardHeader>
          <CardTitle>{t("fps.how.title")}</CardTitle>
        </CardHeader>
        <CardContent>
          <ul className="flex list-disc flex-col gap-2 pl-5 text-sm text-muted">
            <li>{t("fps.how.menu")}</li>
            <li>{t("fps.how.anyGpu")}</li>
            <li>{t("fps.how.untouched")}</li>
            <li>{t("fps.how.potato")}</li>
            <li>{t("fps.how.steam")}</li>
          </ul>
        </CardContent>
      </Card>
    </div>
  );
}
