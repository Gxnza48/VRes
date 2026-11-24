import { Check, Copy, CopyPlus, Play, Star, Trash2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { PageHeader } from "@/components/common/PageHeader";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { Crosshair } from "@/lib/bindings/Crosshair";
import type { Look } from "@/lib/bindings/Look";
import type { ModeRequest } from "@/lib/bindings/ModeRequest";
import type { SavedLook } from "@/lib/bindings/SavedLook";
import type { ShareDecoded } from "@/lib/bindings/ShareDecoded";
import { ipc } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { CrosshairCanvas } from "@/features/crosshair/CrosshairCanvas";
import { commitLook } from "@/features/display/actions";
import { applyMode } from "@/features/resolution/actions";
import { sizeLabel } from "@/features/resolution/format";
import { useCatalogStore } from "@/state/catalog";
import { useEngineStore } from "@/state/engine";
import { saveSettings, useSettingsStore } from "@/state/settings";

type Tab = "looks" | "crosshairs" | "resolutions" | "import";
const TABS: Tab[] = ["looks", "crosshairs", "resolutions", "import"];
const field = "rounded-lg border border-border bg-surface-2 px-2 py-1.5 text-sm text-fg";

/** Botón que copia un código y avisa unos segundos. */
function CopyCode({ encode }: { encode: () => Promise<string | null> }) {
  const { t } = useTranslation();
  const [done, setDone] = useState(false);
  return (
    <Button
      size="sm"
      variant="ghost"
      onClick={() =>
        void encode().then((code) => {
          if (!code) return;
          void navigator.clipboard?.writeText(code).catch(() => undefined);
          setDone(true);
          window.setTimeout(() => setDone(false), 2500);
        })
      }
    >
      {done ? <Check aria-hidden /> : <Copy aria-hidden />}
      {done ? t("share.copied") : t("library.copyCode")}
    </Button>
  );
}

/** Pestaña Biblioteca: todo lo tuyo y lo incluido, con búsqueda y códigos. */
export function LibraryPage() {
  const { t } = useTranslation();
  const settings = useSettingsStore((store) => store.settings);
  const displays = useEngineStore((store) => store.displays);
  const { lookPresets, crosshairPresets, games, load } = useCatalogStore();
  const [tab, setTab] = useState<Tab>("looks");
  const [search, setSearch] = useState("");
  const [game, setGame] = useState("");
  const [imported, setImported] = useState<ShareDecoded | null>(null);
  const [importName, setImportName] = useState("");
  const [importDisplay, setImportDisplay] = useState("");

  useEffect(() => load(), [load]);

  const savedLooks = useMemo(() => settings?.library.looks ?? [], [settings]);
  const query = search.trim().toLowerCase();
  const gameName = (id: string | null) =>
    id ? (games.find((item) => item.id === id)?.name ?? id) : t("looks.anyGame");

  const looks = useMemo(() => {
    const rows = [
      ...[...savedLooks]
        .sort((a, b) => Number(b.favorite) - Number(a.favorite))
        .map((item) => ({ key: item.id, name: item.name, game: item.game, look: item.look, saved: item })),
      ...lookPresets.map((preset) => ({
        key: preset.id,
        name: t(`looks.presets.${preset.id}`),
        game: preset.game,
        look: preset.look,
        saved: null as SavedLook | null,
      })),
    ];
    return rows.filter(
      (row) =>
        (!query || row.name.toLowerCase().includes(query)) && (!game || row.game === game),
    );
  }, [savedLooks, lookPresets, query, game, t]);

  if (!settings) {
    return <PageHeader title={t("library.title")} subtitle={t("library.subtitle")} />;
  }

  const saveLooks = (next: SavedLook[]) =>
    void saveSettings({ ...settings, library: { ...settings.library, looks: next } });
  const applyCrosshair = (crosshair: Crosshair) =>
    void saveSettings({ ...settings, crosshair: { ...settings.crosshair, current: crosshair } });
  const addLook = (name: string, look: Look, gameId: string | null) =>
    saveLooks([
      { id: crypto.randomUUID(), name, game: gameId, favorite: false, look },
      ...savedLooks,
    ]);

  const crosshairs = [
    ...settings.crosshair.saved.map((item, index) => ({
      key: `saved-${index}`,
      name: item.name,
      crosshair: item.crosshair,
      index,
    })),
    ...crosshairPresets.map((preset) => ({
      key: preset.id,
      name: t(`crosshair.presets.${preset.id}`),
      crosshair: preset.crosshair,
      index: -1,
    })),
  ].filter((row) => !query || row.name.toLowerCase().includes(query));

  const confirmed = settings.resolution.confirmed;
  const displayName = (id: string) => {
    const display = displays.find((item) => item.id === id);
    return display ? display.friendlyName || display.gdiName : t("library.disconnected");
  };

  const importMode: ModeRequest | null = imported?.resolution ?? null;

  return (
    <div className="flex flex-col gap-6">
      <PageHeader title={t("library.title")} subtitle={t("library.subtitle")} />

      <div role="tablist" aria-label={t("library.title")} className="flex flex-wrap gap-2">
        {TABS.map((item) => (
          <button
            key={item}
            type="button"
            role="tab"
            aria-selected={tab === item}
            onClick={() => setTab(item)}
            className={cn(
              "rounded-xl border px-3 py-2 text-sm transition-colors",
              tab === item
                ? "border-accent bg-accent font-semibold text-accent-fg"
                : "border-border bg-surface text-muted hover:text-fg",
            )}
          >
            {t(`library.tabs.${item}`)}
          </button>
        ))}
      </div>

      {(tab === "looks" || tab === "crosshairs") && (
        <div className="flex flex-wrap items-center gap-2">
          <input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder={t("library.search")}
            aria-label={t("library.search")}
            className={cn(field, "w-64")}
          />
          {tab === "looks" && (
            <select
              value={game}
              aria-label={t("looks.game")}
              onChange={(event) => setGame(event.target.value)}
              className={field}
            >
              <option value="">{t("looks.allGames")}</option>
              {games.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.name}
                </option>
              ))}
            </select>
          )}
        </div>
      )}

      {tab === "looks" && (
        <Card>
          <CardContent className="gap-0 p-0">
            {looks.length === 0 ? (
              <p className="p-6 text-sm text-muted">{t("library.empty")}</p>
            ) : (
              <ul className="divide-y divide-border">
                {looks.map((row) => (
                  <li key={row.key} className="flex flex-wrap items-center gap-3 px-5 py-3">
                    <div className="flex min-w-0 flex-1 flex-col">
                      <span className="truncate text-sm font-medium text-fg">{row.name}</span>
                      <span className="text-xs text-muted">
                        {gameName(row.game)} · {row.saved ? t("looks.mine") : t("library.included")}
                      </span>
                    </div>
                    <Button size="sm" variant="primary" onClick={() => commitLook(row.look)}>
                      <Play aria-hidden />
                      {t("library.apply")}
                    </Button>
                    <CopyCode encode={() => ipc.shareEncodeLook(row.look)} />
                    {row.saved ? (
                      <>
                        <Button
                          size="sm"
                          variant="ghost"
                          aria-label={t("looks.favorite")}
                          aria-pressed={row.saved.favorite}
                          onClick={() =>
                            saveLooks(
                              savedLooks.map((item) =>
                                item.id === row.saved?.id ? { ...item, favorite: !item.favorite } : item,
                              ),
                            )
                          }
                        >
                          <Star aria-hidden fill={row.saved.favorite ? "currentColor" : "none"} />
                        </Button>
                        <Button
                          size="sm"
                          variant="ghost"
                          aria-label={t("looks.remove")}
                          onClick={() => saveLooks(savedLooks.filter((item) => item.id !== row.saved?.id))}
                        >
                          <Trash2 aria-hidden />
                        </Button>
                      </>
                    ) : (
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => addLook(row.name, row.look, row.game)}
                      >
                        <CopyPlus aria-hidden />
                        {t("library.duplicate")}
                      </Button>
                    )}
                  </li>
                ))}
              </ul>
            )}
          </CardContent>
        </Card>
      )}

      {tab === "crosshairs" && (
        <Card>
          <CardContent className="gap-0 p-0">
            <ul className="divide-y divide-border">
              {crosshairs.map((row) => (
                <li key={row.key} className="flex flex-wrap items-center gap-3 px-5 py-3">
                  <CrosshairCanvas crosshair={row.crosshair} box={44} zoom={2} className="border-0" />
                  <div className="flex min-w-0 flex-1 flex-col">
                    <span className="truncate text-sm font-medium text-fg">{row.name}</span>
                    <span className="text-xs text-muted">
                      {row.index >= 0 ? t("looks.mine") : t("library.included")}
                    </span>
                  </div>
                  <Button size="sm" variant="primary" onClick={() => applyCrosshair(row.crosshair)}>
                    <Play aria-hidden />
                    {t("library.use")}
                  </Button>
                  <CopyCode encode={() => ipc.shareEncodeCrosshair(row.crosshair)} />
                  {row.index >= 0 && (
                    <Button
                      size="sm"
                      variant="ghost"
                      aria-label={t("looks.remove")}
                      onClick={() =>
                        void saveSettings({
                          ...settings,
                          crosshair: {
                            ...settings.crosshair,
                            saved: settings.crosshair.saved.filter((_, position) => position !== row.index),
                          },
                        })
                      }
                    >
                      <Trash2 aria-hidden />
                    </Button>
                  )}
                </li>
              ))}
            </ul>
          </CardContent>
        </Card>
      )}

      {tab === "resolutions" && (
        <Card>
          <CardHeader>
            <CardTitle>{t("library.tabs.resolutions")}</CardTitle>
            <CardDescription>{t("library.resolutionsHint")}</CardDescription>
          </CardHeader>
          <CardContent className="gap-0 p-0">
            {confirmed.length === 0 ? (
              <p className="px-6 pb-6 text-sm text-muted">{t("library.resolutionsEmpty")}</p>
            ) : (
              <ul className="divide-y divide-border">
                {confirmed.map((item) => (
                  <li key={`${item.displayId}-${item.mode.width}x${item.mode.height}@${item.mode.refreshHz}`} className="flex flex-wrap items-center gap-3 px-5 py-3">
                    <div className="flex min-w-0 flex-1 flex-col">
                      <span className="font-display text-sm font-bold text-fg">
                        {sizeLabel(item.mode.width, item.mode.height)} · {item.mode.refreshHz} Hz
                      </span>
                      <span className="text-xs text-muted">
                        {displayName(item.displayId)} · {t(`resolution.scaling.${item.mode.scaling}`)}
                      </span>
                    </div>
                    <Button
                      size="sm"
                      variant="primary"
                      disabled={!displays.some((display) => display.id === item.displayId)}
                      onClick={() => void applyMode(item.displayId, item.mode)}
                    >
                      <Play aria-hidden />
                      {t("library.apply")}
                    </Button>
                    <CopyCode encode={() => ipc.shareEncodeResolution(item.mode)} />
                  </li>
                ))}
              </ul>
            )}
          </CardContent>
        </Card>
      )}

      {tab === "import" && (
        <Card>
          <CardHeader>
            <CardTitle>{t("library.import.title")}</CardTitle>
            <CardDescription>{t("library.import.description")}</CardDescription>
          </CardHeader>
          <CardContent className="gap-4">
            <ImportBox
              onDecoded={(decoded) => {
                setImported(decoded);
                setImportName("");
              }}
            />
            {imported?.look && (
              <form
                className="flex flex-wrap items-center gap-2"
                onSubmit={(event) => {
                  event.preventDefault();
                  if (!imported.look) return;
                  addLook(importName.trim() || t("library.import.defaultName"), imported.look, null);
                  commitLook(imported.look);
                  setImported(null);
                }}
              >
                <span className="text-sm text-fg">{t("library.import.found.look")}</span>
                <input
                  value={importName}
                  maxLength={40}
                  placeholder={t("looks.name")}
                  aria-label={t("looks.name")}
                  onChange={(event) => setImportName(event.target.value)}
                  className={cn(field, "w-52")}
                />
                <Button type="submit" variant="primary">
                  {t("library.import.saveLook")}
                </Button>
              </form>
            )}
            {imported?.crosshair && (
              <form
                className="flex flex-wrap items-center gap-2"
                onSubmit={(event) => {
                  event.preventDefault();
                  const crosshair = imported.crosshair;
                  if (!crosshair) return;
                  void saveSettings({
                    ...settings,
                    crosshair: {
                      ...settings.crosshair,
                      current: crosshair,
                      saved: [
                        { name: importName.trim() || t("library.import.defaultName"), crosshair },
                        ...settings.crosshair.saved,
                      ],
                    },
                  });
                  setImported(null);
                }}
              >
                <CrosshairCanvas crosshair={imported.crosshair} box={44} zoom={2} className="border-0" />
                <span className="text-sm text-fg">{t("library.import.found.crosshair")}</span>
                <input
                  value={importName}
                  maxLength={40}
                  placeholder={t("looks.name")}
                  aria-label={t("looks.name")}
                  onChange={(event) => setImportName(event.target.value)}
                  className={cn(field, "w-52")}
                />
                <Button type="submit" variant="primary">
                  {t("library.import.saveCrosshair")}
                </Button>
              </form>
            )}
            {importMode && (
              <div className="flex flex-wrap items-center gap-2">
                <span className="text-sm text-fg">
                  {t("library.import.found.resolution", {
                    mode: `${sizeLabel(importMode.width, importMode.height)} · ${importMode.refreshHz} Hz`,
                  })}
                </span>
                <select
                  value={importDisplay}
                  aria-label={t("crosshair.placement.display")}
                  onChange={(event) => setImportDisplay(event.target.value)}
                  className={field}
                >
                  <option value="">—</option>
                  {displays.map((display, index) => (
                    <option key={display.id} value={display.id}>
                      {index + 1} · {display.friendlyName || display.gdiName}
                    </option>
                  ))}
                </select>
                <Button
                  variant="primary"
                  disabled={!importDisplay}
                  onClick={() => {
                    void applyMode(importDisplay, importMode);
                    setImported(null);
                  }}
                >
                  {t("library.import.applyOn")}
                </Button>
              </div>
            )}
          </CardContent>
        </Card>
      )}
    </div>
  );
}

/** Pegar cualquier código de VRes (look, crosshair o resolución). */
function ImportBox({ onDecoded }: { onDecoded: (decoded: ShareDecoded) => void }) {
  const { t } = useTranslation();
  const [code, setCode] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  return (
    <form
      className="flex flex-col gap-2"
      onSubmit={(event) => {
        event.preventDefault();
        setProblem(null);
        void ipc.shareDecode(code).then((decoded) => {
          if (decoded.error) {
            setProblem(t(`share.errors.${decoded.error}`));
            return;
          }
          setCode("");
          onDecoded(decoded);
        });
      }}
    >
      <div className="flex flex-wrap items-center gap-2">
        <input
          value={code}
          onChange={(event) => setCode(event.target.value)}
          placeholder={t("library.import.placeholder")}
          aria-label={t("share.paste")}
          spellCheck={false}
          className={cn(field, "min-w-64 flex-1 font-display tracking-wide")}
        />
        <Button type="submit" variant="primary" disabled={!code.trim()}>
          {t("library.import.read")}
        </Button>
      </div>
      {problem && (
        <p role="alert" className="text-sm text-danger">
          {problem}
        </p>
      )}
    </form>
  );
}

