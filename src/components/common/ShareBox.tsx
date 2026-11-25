import { Check, ClipboardPaste, Copy } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import type { ShareDecoded } from "@/lib/bindings/ShareDecoded";
import type { ShareKind } from "@/lib/bindings/ShareKind";
import { ipc } from "@/lib/ipc";
import { cn } from "@/lib/utils";

interface ShareBoxProps {
  /** Qué tipo de código se espera al pegar. */
  kind: ShareKind;
  /** Arma el código de lo actual (`null` si no se puede compartir). */
  encode: () => Promise<string | null>;
  /** Llegó un código válido del tipo esperado. */
  onDecoded: (decoded: ShareDecoded) => void;
}

/**
 * Copiar el código de lo actual y pegar el de un amigo. Los códigos son
 * texto: sirven por Discord o WhatsApp y toleran espacios y errores comunes.
 */
export function ShareBox({ kind, encode, onDecoded }: ShareBoxProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState<string | null>(null);
  const [copyFailed, setCopyFailed] = useState(false);
  const [pasted, setPasted] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  const [clamped, setClamped] = useState(false);
  // La mira vino de un código de VALORANT o CS2 (traducida).
  const [imported, setImported] = useState<string | null>(null);

  const copy = async () => {
    setCopyFailed(false);
    const code = await encode().catch(() => null);
    if (!code) {
      setCopyFailed(true);
      return;
    }
    setCopied(code);
    await navigator.clipboard?.writeText(code).catch(() => undefined);
    window.setTimeout(() => setCopied(null), 4000);
  };

  const read = async () => {
    setProblem(null);
    setClamped(false);
    setImported(null);
    const decoded = await ipc.shareDecode(pasted).catch(() => null);
    if (!decoded) {
      setProblem(t("share.errors.badPrefix"));
      return;
    }
    if (decoded.error) {
      setProblem(t(`share.errors.${decoded.error}`));
      return;
    }
    if (decoded.kind !== kind) {
      setProblem(t("share.errors.wrongKind"));
      return;
    }
    setClamped(decoded.clamped);
    setImported(decoded.source ? t(`share.games.${decoded.source}`) : null);
    setPasted("");
    onDecoded(decoded);
  };

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-3">
        <Button onClick={() => void copy()}>
          {copied ? <Check aria-hidden /> : <Copy aria-hidden />}
          {copied ? t("share.copied") : t("share.copy")}
        </Button>
        {copied && (
          <code className="rounded-lg border border-border bg-surface-2 px-2 py-1 font-display text-xs tracking-wide text-accent select-all">
            {copied}
          </code>
        )}
        {copyFailed && <span className="text-sm text-warning">{t("share.notShareable")}</span>}
      </div>
      <form
        className="flex flex-wrap items-center gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          if (pasted.trim()) void read();
        }}
      >
        <input
          value={pasted}
          onChange={(event) => setPasted(event.target.value)}
          placeholder={t(`share.placeholder.${kind}`)}
          aria-label={t("share.paste")}
          spellCheck={false}
          className={cn(
            "min-w-64 flex-1 rounded-lg border bg-surface-2 px-3 py-1.5 font-display text-sm tracking-wide text-fg",
            problem ? "border-danger" : "border-border",
          )}
        />
        <Button type="submit" variant="primary" disabled={!pasted.trim()}>
          <ClipboardPaste aria-hidden />
          {t("share.apply")}
        </Button>
      </form>
      {problem && (
        <p role="alert" className="text-sm text-danger">
          {problem}
        </p>
      )}
      {clamped && <p className="text-sm text-warning">{t("share.clamped")}</p>}
      {imported && (
        <p role="status" className="text-sm text-success">
          {t("share.imported", { game: imported })}
        </p>
      )}
    </div>
  );
}
