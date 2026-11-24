import { Keyboard, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import type { Hotkey } from "@/lib/bindings/Hotkey";
import type { HotkeyIssue } from "@/lib/bindings/HotkeyIssue";
import { hotkeyLabel, MODIFIER_CODES } from "@/lib/hotkeys";
import { ipc } from "@/lib/ipc";
import { cn } from "@/lib/utils";

interface HotkeyRecorderProps {
  value: Hotkey | null;
  disabled?: boolean;
  onChange: (hotkey: Hotkey | null) => void;
}

interface Problem {
  kind: "error" | "warning";
  issue: HotkeyIssue;
}

/**
 * Graba un atajo: se aprieta la combinación y queda guardada la tecla física
 * con sus modificadores. Mientras graba, el atajo actual queda suspendido
 * (si no, Windows lo atraparía antes de que llegue acá).
 */
export function HotkeyRecorder({ value, disabled, onChange }: HotkeyRecorderProps) {
  const { t } = useTranslation();
  const [recording, setRecording] = useState(false);
  const [problem, setProblem] = useState<Problem | null>(null);

  useEffect(() => {
    if (!recording) return;
    void ipc.hotkeysSuspend(true).catch(() => undefined);

    const onKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
      if (event.code === "Escape") {
        setRecording(false);
        return;
      }
      if (MODIFIER_CODES.has(event.code)) return;
      const hotkey: Hotkey = {
        code: event.code,
        ctrl: event.ctrlKey,
        alt: event.altKey,
        shift: event.shiftKey,
        win: event.metaKey,
      };
      ipc
        .hotkeysCheck(hotkey)
        .then((check) => {
          if (check.error) {
            setProblem({ kind: "error", issue: check.error });
            return;
          }
          setProblem(check.warning ? { kind: "warning", issue: check.warning } : null);
          setRecording(false);
          onChange(hotkey);
        })
        .catch((error: unknown) => console.error("hotkeys_check", error));
    };
    // Que Alt suelto no active el menú de la ventana.
    const onKeyUp = (event: KeyboardEvent) => event.preventDefault();

    window.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("keyup", onKeyUp, true);
    return () => {
      window.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("keyup", onKeyUp, true);
      void ipc.hotkeysSuspend(false).catch(() => undefined);
    };
  }, [recording, onChange]);

  return (
    <div className="flex flex-col gap-2">
      <div className="flex flex-wrap items-center gap-2">
        <span
          aria-live="polite"
          className={cn(
            "inline-flex min-h-9 min-w-40 items-center gap-2 rounded-lg border px-3 text-sm",
            recording
              ? "border-accent bg-accent/10 text-fg"
              : value
                ? "border-border bg-surface-2 font-display font-bold tracking-wide text-accent"
                : "border-border bg-surface-2 text-muted",
          )}
        >
          <Keyboard aria-hidden className="size-4 shrink-0 text-muted" />
          {recording
            ? t("resolution.hotkey.recording")
            : value
              ? hotkeyLabel(value, (key) => t(key))
              : t("resolution.hotkey.none")}
        </span>
        <Button
          size="sm"
          disabled={disabled}
          onClick={() => {
            setProblem(null);
            setRecording((current) => !current);
          }}
        >
          {recording
            ? t("resolution.hotkey.cancel")
            : value
              ? t("resolution.hotkey.change")
              : t("resolution.hotkey.set")}
        </Button>
        {value && !recording && (
          <Button
            size="sm"
            variant="ghost"
            disabled={disabled}
            onClick={() => {
              setProblem(null);
              onChange(null);
            }}
          >
            <X aria-hidden />
            {t("resolution.hotkey.clear")}
          </Button>
        )}
      </div>
      {problem && (
        <p
          role={problem.kind === "error" ? "alert" : undefined}
          className={cn("text-xs", problem.kind === "error" ? "text-danger" : "text-warning")}
        >
          {t(`resolution.hotkey.issues.${problem.issue}`)}
        </p>
      )}
    </div>
  );
}
