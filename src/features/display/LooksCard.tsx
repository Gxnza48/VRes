import { Save, Star, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { Look } from "@/lib/bindings/Look";
import type { SavedLook } from "@/lib/bindings/SavedLook";
import { cn } from "@/lib/utils";
import { useCatalogStore } from "@/state/catalog";
import { saveSettings, useSettingsStore } from "@/state/settings";

import { commitLook } from "./actions";

const field = "rounded-lg border border-border bg-surface-2 px-2 py-1.5 text-sm text-fg";

interface Tile {
  key: string;
  name: string;
  game: string | null;
  look: Look;
  saved: SavedLook | null;
}

/** Looks incluidos y guardados, con filtro por juego y "guardar el actual". */
export function LooksCard({ look, enabled }: { look: Look; enabled: boolean }) {
  const { t } = useTranslation();
  const settings = useSettingsStore((store) => store.settings);
  const { lookPresets, games, load } = useCatalogStore();
  const [game, setGame] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [saveGame, setSaveGame] = useState<string>("");

  useEffect(() => load(), [load]);

  const saved = settings?.library.looks ?? [];
  const gameName = (id: string | null) => games.find((item) => item.id === id)?.name ?? null;
  const tiles: Tile[] = [
    ...[...saved]
      .sort((a, b) => Number(b.favorite) - Number(a.favorite))
      .map((item) => ({
        key: `saved-${item.id}`,
        name: item.name,
        game: item.game,
        look: item.look,
        saved: item,
      })),
    ...lookPresets.map((preset) => ({
      key: `preset-${preset.id}`,
      name: t(`looks.presets.${preset.id}`),
      game: preset.game,
      look: preset.look,
      saved: null,
    })),
  ].filter((tile) => game === null || tile.game === game || tile.game === null);

  const updateSaved = (looks: SavedLook[]) => {
    if (!settings) return;
    void saveSettings({ ...settings, library: { ...settings.library, looks } });
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("looks.title")}</CardTitle>
        <CardDescription>{t("looks.description")}</CardDescription>
      </CardHeader>
      <CardContent className="gap-4">
        <div className="flex flex-wrap gap-2">
          {[null, ...games.map((item) => item.id)].map((id) => (
            <button
              key={id ?? "all"}
              type="button"
              aria-pressed={game === id}
              onClick={() => setGame(id)}
              className={cn(
                "rounded-lg border px-2.5 py-1 text-xs transition-colors",
                game === id
                  ? "border-accent bg-accent font-semibold text-accent-fg"
                  : "border-border bg-surface-2 text-muted hover:text-fg",
              )}
            >
              {id === null ? t("looks.allGames") : gameName(id)}
            </button>
          ))}
        </div>

        <div className="grid grid-cols-2 gap-2 md:grid-cols-4">
          {tiles.map((tile) => (
            <div
              key={tile.key}
              className="group flex items-center gap-1 rounded-xl border border-border bg-surface-2 transition-colors hover:border-accent"
            >
              <button
                type="button"
                disabled={!enabled}
                onClick={() => void commitLook(tile.look)}
                className="flex min-w-0 flex-1 flex-col items-start gap-0.5 px-3 py-2 text-left disabled:opacity-50"
              >
                <span className="w-full truncate text-sm font-medium text-fg">{tile.name}</span>
                <span className="text-xs text-muted">
                  {tile.game ? gameName(tile.game) : t("looks.anyGame")}
                  {tile.saved ? ` · ${t("looks.mine")}` : ""}
                </span>
              </button>
              {tile.saved && (
                <div className="flex flex-col pr-1">
                  <button
                    type="button"
                    aria-label={t("looks.favorite")}
                    aria-pressed={tile.saved.favorite}
                    onClick={() =>
                      updateSaved(
                        saved.map((item) =>
                          item.id === tile.saved?.id ? { ...item, favorite: !item.favorite } : item,
                        ),
                      )
                    }
                    className={cn("p-1", tile.saved.favorite ? "text-accent" : "text-muted hover:text-fg")}
                  >
                    <Star aria-hidden className="size-3.5" fill={tile.saved.favorite ? "currentColor" : "none"} />
                  </button>
                  <button
                    type="button"
                    aria-label={t("looks.remove")}
                    onClick={() => updateSaved(saved.filter((item) => item.id !== tile.saved?.id))}
                    className="p-1 text-muted hover:text-danger"
                  >
                    <Trash2 aria-hidden className="size-3.5" />
                  </button>
                </div>
              )}
            </div>
          ))}
        </div>

        <form
          className="flex flex-wrap items-center gap-2"
          onSubmit={(event) => {
            event.preventDefault();
            const trimmed = name.trim();
            if (!trimmed) return;
            updateSaved([
              {
                id: crypto.randomUUID(),
                name: trimmed,
                game: saveGame || null,
                favorite: false,
                look,
              },
              ...saved,
            ]);
            setName("");
          }}
        >
          <input
            value={name}
            maxLength={40}
            placeholder={t("looks.name")}
            aria-label={t("looks.name")}
            onChange={(event) => setName(event.target.value)}
            className={cn(field, "w-52")}
          />
          <select
            value={saveGame}
            aria-label={t("looks.game")}
            onChange={(event) => setSaveGame(event.target.value)}
            className={field}
          >
            <option value="">{t("looks.anyGame")}</option>
            {games.map((item) => (
              <option key={item.id} value={item.id}>
                {item.name}
              </option>
            ))}
          </select>
          <Button type="submit" variant="primary" disabled={!name.trim()}>
            <Save aria-hidden />
            {t("looks.save")}
          </Button>
        </form>
      </CardContent>
    </Card>
  );
}
