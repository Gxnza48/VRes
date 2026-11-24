import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import type { PendingMode } from "@/lib/bindings/PendingMode";

import { keepMode, revertMode } from "./actions";
import { sizeLabel } from "./format";

interface CountdownDialogProps {
  pending: PendingMode;
  busy: boolean;
}

/**
 * "¿Conservar esta resolución?" con cuenta regresiva. Si nadie responde, el
 * motor vuelve solo a la anterior (aunque esta ventana esté cerrada). El foco
 * arranca en "Revertir": un Enter a ciegas no deja una pantalla rota.
 */
export function CountdownDialog({ pending, busy }: CountdownDialogProps) {
  const { t } = useTranslation();
  // El reloj se toma en el efecto (renderizar tiene que ser puro).
  const deadline = useRef(0);
  const [left, setLeft] = useState(() => Math.ceil(pending.expiresInMs / 1000));
  const revertButton = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    deadline.current = Date.now() + pending.expiresInMs;
    const tick = () => setLeft(Math.max(0, Math.ceil((deadline.current - Date.now()) / 1000)));
    tick();
    const timer = window.setInterval(tick, 250);
    revertButton.current?.focus();
    return () => window.clearInterval(timer);
  }, [pending.token, pending.expiresInMs]);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-bg/80 p-6">
      <div
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="pending-title"
        aria-describedby="pending-body"
        onKeyDown={(event) => {
          if (event.key === "Escape" && !busy) void revertMode(pending.token);
        }}
        className="flex w-full max-w-md flex-col items-center gap-4 rounded-card border border-accent/50 bg-surface p-8 text-center"
      >
        <h2 id="pending-title" className="font-display text-xl font-black tracking-wide text-fg">
          {t("resolution.pending.title")}
        </h2>
        <p className="font-display text-sm font-bold text-muted">
          {sizeLabel(pending.mode.width, pending.mode.height)} · {pending.mode.refreshHz} Hz ·{" "}
          {t(`resolution.scaling.${pending.mode.scaling}`)}
        </p>
        <p
          aria-live="polite"
          className="font-display text-6xl font-black tabular-nums text-accent"
        >
          {left}
        </p>
        <p id="pending-body" className="text-sm text-muted">
          {t("resolution.pending.body")}
        </p>
        {pending.reason && (
          <p className="text-sm text-warning">{t(`reasons.${pending.reason}`)}</p>
        )}
        <div className="mt-2 flex w-full gap-3">
          <Button
            variant="primary"
            size="lg"
            className="flex-1"
            disabled={busy}
            onClick={() => void keepMode(pending.token)}
          >
            {t("resolution.pending.keep")}
          </Button>
          <Button
            ref={revertButton}
            size="lg"
            className="flex-1"
            disabled={busy}
            onClick={() => void revertMode(pending.token)}
          >
            {t("resolution.pending.revert")}
          </Button>
        </div>
      </div>
    </div>
  );
}
