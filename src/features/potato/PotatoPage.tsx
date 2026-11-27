import { Cpu, ShieldAlert, ShieldCheck } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { PageHeader } from "@/components/common/PageHeader";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { ErrorCode } from "@/lib/bindings/ErrorCode";
import type { PotatoLevel } from "@/lib/bindings/PotatoLevel";
import type { PotatoStatus } from "@/lib/bindings/PotatoStatus";
import { ipc, isIpcError } from "@/lib/ipc";
import { cn } from "@/lib/utils";

const LEVELS: PotatoLevel[] = ["off", "low", "potato"];

/** Pestaña Potato: texturas planas para más FPS en placas NVIDIA. */
export function PotatoPage() {
  const { t } = useTranslation();
  const [games, setGames] = useState<PotatoStatus[] | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<ErrorCode | null>(null);
  const [done, setDone] = useState<string | null>(null);

  useEffect(() => {
    ipc
      .potatoStatus()
      .then(setGames)
      .catch(() => setGames([]));
  }, []);

  const apply = (game: string, level: PotatoLevel) => {
    setBusy(game);
    setError(null);
    setDone(null);
    ipc
      .potatoApply(game, level)
      .then((next) => {
        setGames(next);
        setDone(game);
      })
      .catch((reason: unknown) => setError(isIpcError(reason) ? reason.code : "internal"))
      .finally(() => setBusy(null));
  };

  const available = games?.some((game) => game.available) ?? false;

  return (
    <div className="flex flex-col gap-6">
      <PageHeader title={t("potato.title")} subtitle={t("potato.subtitle")} />

      <p className="flex items-start gap-3 rounded-xl border border-warning/40 bg-warning/10 px-4 py-3 text-sm text-fg">
        <ShieldAlert aria-hidden className="mt-0.5 size-4 shrink-0 text-warning" />
        {t("potato.warning")}
      </p>
      {error && (
        <p role="alert" className="text-sm text-danger">
          {t(`errors.${error}`)}
        </p>
      )}

      {games !== null && !available && (
        <Card>
          <CardHeader>
            <CardTitle>{t("potato.unavailable.title")}</CardTitle>
            <CardDescription>{t("potato.unavailable.description")}</CardDescription>
          </CardHeader>
        </Card>
      )}

      {available &&
        games?.map((status) => (
          <Card key={status.game.id}>
            <CardHeader className="flex-row items-start justify-between gap-4">
              <div className="flex flex-col gap-1">
                <CardTitle className="flex items-center gap-2">
                  <Cpu aria-hidden className="size-4 text-accent" />
                  {status.game.name}
                </CardTitle>
                <CardDescription>{t("potato.game.description", { exe: status.game.executable })}</CardDescription>
              </div>
              <Badge tone={status.level === "off" ? undefined : "accent"}>
                {t(`potato.levels.${status.level}.name`)}
              </Badge>
            </CardHeader>
            <CardContent className="gap-4">
              <div role="radiogroup" aria-label={status.game.name} className="grid gap-3 md:grid-cols-3">
                {LEVELS.map((level) => {
                  const selected = status.level === level;
                  return (
                    <button
                      key={level}
                      type="button"
                      role="radio"
                      aria-checked={selected}
                      disabled={busy !== null}
                      onClick={() => apply(status.game.id, level)}
                      className={cn(
                        "flex flex-col gap-1 rounded-xl border p-4 text-left transition-colors disabled:opacity-60",
                        selected
                          ? "border-accent bg-accent/10"
                          : "border-border bg-surface-2 hover:border-muted",
                      )}
                    >
                      <span className="text-sm font-semibold text-fg">
                        {t(`potato.levels.${level}.name`)}
                      </span>
                      <span className="text-xs text-muted">{t(`potato.levels.${level}.hint`)}</span>
                    </button>
                  );
                })}
              </div>
              {busy === status.game.id && <p className="text-sm text-muted">{t("potato.applying")}</p>}
              {done === status.game.id && (
                <p className="flex items-center gap-2 text-sm text-success">
                  <ShieldCheck aria-hidden className="size-4" />
                  {t("potato.restart")}
                </p>
              )}
              {status.custom && <p className="text-xs text-warning">{t("potato.custom")}</p>}
              {status.legacy && <p className="text-xs text-muted">{t("potato.legacy")}</p>}
            </CardContent>
          </Card>
        ))}

      <Card>
        <CardHeader>
          <CardTitle>{t("potato.how.title")}</CardTitle>
        </CardHeader>
        <CardContent>
          <ul className="flex list-disc flex-col gap-2 pl-5 text-sm text-muted">
            <li>{t("potato.how.driver")}</li>
            <li>{t("potato.how.admin")}</li>
            <li>{t("potato.how.hybrid")}</li>
            <li>{t("potato.how.undo")}</li>
          </ul>
        </CardContent>
      </Card>
    </div>
  );
}
