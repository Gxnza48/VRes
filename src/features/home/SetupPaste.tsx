import { ClipboardPaste } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import type { ShareDecoded } from "@/lib/bindings/ShareDecoded";
import { ipc } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { commitLook } from "@/features/display/actions";
import { applyMode } from "@/features/resolution/actions";
import { useEngineStore } from "@/state/engine";
import { saveSettings, useSettingsStore } from "@/state/settings";

/** Los códigos de un texto pegado (uno por línea o separados por espacios). */
function splitCodes(text: string): string[] {
  return text
    .split(/[\s,]+/)
    .map((code) => code.trim())
    .filter((code) => code.length > 0);
}

/**
 * Pega un setup completo (lo que copia «Copiar mi setup»): el look, la mira y
 * la resolución se aplican de una vez. La resolución pide confirmación como
 * siempre, así que si algo sale mal vuelve sola.
 */
export function SetupPaste() {
  const { t } = useTranslation();
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [applied, setApplied] = useState<string[] | null>(null);
  const [failed, setFailed] = useState(0);

  const apply = async () => {
    const codes = splitCodes(text);
    if (codes.length === 0) return;
    setBusy(true);
    setApplied(null);
    const results = await Promise.all(
      codes.map((code) => ipc.shareDecode(code).catch(() => null as ShareDecoded | null)),
    );
    const ok = results.filter((result): result is ShareDecoded => !!result && !result.error);
    setFailed(codes.length - ok.length);
    const done: string[] = [];

    const look = ok.find((result) => result.look)?.look;
    if (look) {
      commitLook(look);
      done.push(t("home.paste.parts.look"));
    }
    const crosshair = ok.find((result) => result.crosshair)?.crosshair;
    const settings = useSettingsStore.getState().settings;
    if (crosshair && settings) {
      await saveSettings({
        ...settings,
        crosshair: { ...settings.crosshair, current: crosshair, enabled: true },
      });
      done.push(t("home.paste.parts.crosshair"));
    }
    const mode = ok.find((result) => result.resolution)?.resolution;
    if (mode) {
      const displays = useEngineStore.getState().displays;
      const target =
        settings?.resolution.toggle?.displayId ??
        displays.find((display) => display.primary)?.id ??
        displays[0]?.id;
      if (target && (await applyMode(target, mode))) {
        done.push(t("home.paste.parts.resolution"));
      }
    }

    setApplied(done);
    if (done.length > 0) setText("");
    setBusy(false);
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <ClipboardPaste aria-hidden className="size-4 text-accent" />
          {t("home.paste.title")}
        </CardTitle>
        <CardDescription>{t("home.paste.description")}</CardDescription>
      </CardHeader>
      <CardContent className="gap-3">
        <textarea
          value={text}
          onChange={(event) => setText(event.target.value)}
          rows={3}
          spellCheck={false}
          placeholder={"VRL1-…\nVRC1-…\nVRR1-…"}
          aria-label={t("home.paste.title")}
          className={cn(
            "w-full resize-none rounded-lg border bg-surface-2 px-3 py-2 font-display text-xs tracking-wide text-fg",
            failed > 0 && applied?.length === 0 ? "border-danger" : "border-border",
          )}
        />
        <Button
          variant="primary"
          className="self-start"
          disabled={busy || splitCodes(text).length === 0}
          onClick={() => void apply()}
        >
          <ClipboardPaste aria-hidden />
          {t("home.paste.apply")}
        </Button>
        {applied && applied.length > 0 && (
          <p role="status" className="text-sm text-success">
            {t("home.paste.done", { parts: applied.join(", ") })}
          </p>
        )}
        {applied && failed > 0 && (
          <p role="alert" className="text-sm text-warning">
            {t("home.paste.failed", { count: failed })}
          </p>
        )}
      </CardContent>
    </Card>
  );
}
