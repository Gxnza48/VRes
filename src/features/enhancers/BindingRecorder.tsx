import { Keyboard, Mouse, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import type { Binding } from "@/lib/bindings/Binding";
import { ipc } from "@/lib/ipc";
import { cn } from "@/lib/utils";

import { bindingLabel, MOUSE_BUTTONS } from "./binding";

interface BindingRecorderProps {
  id: string;
  value: Binding;
  onChange: (binding: Binding) => void;
}

/**
 * Graba la tecla o el botón del mouse de Dark Boost o Zoom. Mientras graba,
 * el motor deja de escuchar sus teclas (si no, se activarían al grabarlas).
 */
export function BindingRecorder({ id, value, onChange }: BindingRecorderProps) {
  const { t } = useTranslation();
  const [recording, setRecording] = useState(false);
  const [unsupported, setUnsupported] = useState(false);
  // El efecto de grabación no se rearma si el padre pasa otra función.
  const onChangeRef = useRef(onChange);
  useEffect(() => {
    onChangeRef.current = onChange;
  }, [onChange]);

  useEffect(() => {
    if (!recording) return;
    void ipc.hotkeysSuspend(true).catch(() => undefined);

    const finish = (binding: Binding) => {
      setRecording(false);
      setUnsupported(false);
      onChangeRef.current(binding);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
      if (event.repeat) return;
      if (event.code === "Escape") {
        setRecording(false);
        return;
      }
      const code = event.code;
      ipc
        .enhancersKeySupported(code)
        .then((supported) => {
          if (supported) finish({ kind: "key", code });
          else setUnsupported(true);
        })
        .catch(() => setUnsupported(true));
    };
    const onMouseDown = (event: MouseEvent) => {
      const button = MOUSE_BUTTONS[event.button];
      if (!button) return;
      event.preventDefault();
      event.stopPropagation();
      finish({ kind: "mouse", button });
    };
    // Que el botón derecho no abra el menú ni los laterales naveguen.
    const swallow = (event: MouseEvent) => {
      if (event.button !== 0) event.preventDefault();
    };
    const noMenu = (event: Event) => event.preventDefault();

    window.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("mousedown", onMouseDown, true);
    window.addEventListener("mouseup", swallow, true);
    window.addEventListener("auxclick", swallow, true);
    window.addEventListener("contextmenu", noMenu, true);
    return () => {
      window.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("mousedown", onMouseDown, true);
      window.removeEventListener("mouseup", swallow, true);
      window.removeEventListener("auxclick", swallow, true);
      window.removeEventListener("contextmenu", noMenu, true);
      void ipc.hotkeysSuspend(false).catch(() => undefined);
    };
  }, [recording]);

  const Icon = value.kind === "mouse" ? Mouse : Keyboard;

  return (
    <div className="flex flex-col gap-1.5 text-sm">
      <span id={`${id}-label`} className="text-muted">
        {t("enhancers.binding.label")}
      </span>
      <div className="flex flex-wrap items-center gap-2">
        <span
          aria-labelledby={`${id}-label`}
          className={cn(
            "inline-flex min-h-9 items-center gap-2 rounded-lg border px-3 py-1.5",
            recording ? "border-accent bg-accent/10 text-fg" : "border-border bg-surface-2 text-fg",
          )}
        >
          <Icon aria-hidden className="size-4 text-muted" />
          {recording ? t("enhancers.binding.recording") : bindingLabel(value, t)}
        </span>
        {recording ? (
          <Button size="sm" variant="ghost" onClick={() => setRecording(false)}>
            <X aria-hidden />
            {t("enhancers.binding.cancel")}
          </Button>
        ) : (
          <Button
            size="sm"
            variant="secondary"
            onClick={() => {
              setUnsupported(false);
              setRecording(true);
            }}
          >
            {t("enhancers.binding.change")}
          </Button>
        )}
      </div>
      {recording && <span className="text-xs text-muted">{t("enhancers.binding.hint")}</span>}
      {unsupported && (
        <span role="alert" className="text-xs text-warning">
          {t("enhancers.binding.unsupported")}
        </span>
      )}
    </div>
  );
}
