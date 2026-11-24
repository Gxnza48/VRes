import { useEffect, useRef, useState } from "react";

import type { Crosshair } from "@/lib/bindings/Crosshair";
import { ipc } from "@/lib/ipc";
import { cn } from "@/lib/utils";

import { parseRendered } from "./render";

interface CrosshairCanvasProps {
  crosshair: Crosshair;
  /** Lado del recuadro en píxeles CSS. */
  box: number;
  /** Aumento (1 = tamaño real en un monitor sin escala). */
  zoom?: number;
  className?: string;
  label?: string;
}

/**
 * Pinta el crosshair dibujado por el renderer de Rust (el mismo que se ve
 * sobre el juego), centrado en un recuadro oscuro.
 */
export function CrosshairCanvas({ crosshair, box, zoom = 1, className, label }: CrosshairCanvasProps) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let cancelled = false;
    ipc
      .crosshairRender(crosshair)
      .then((buffer) => {
        if (cancelled) return;
        const rendered = parseRendered(buffer);
        const element = canvas.current;
        const context = element?.getContext("2d");
        setFailed(rendered === null);
        if (!rendered || !element || !context) return;
        const ratio = window.devicePixelRatio || 1;
        element.width = Math.round(box * ratio);
        element.height = Math.round(box * ratio);
        context.clearRect(0, 0, element.width, element.height);
        const image = new ImageData(new Uint8ClampedArray(rendered.rgba), rendered.size, rendered.size);
        const scratch = document.createElement("canvas");
        scratch.width = rendered.size;
        scratch.height = rendered.size;
        scratch.getContext("2d")?.putImageData(image, 0, 0);
        // Un píxel del dibujo = un píxel físico × zoom, sin suavizar.
        const scale = zoom;
        context.imageSmoothingEnabled = false;
        const drawn = rendered.size * scale;
        const middle = element.width / 2;
        context.drawImage(
          scratch,
          Math.round(middle - (rendered.center + 0.5) * scale),
          Math.round(middle - (rendered.center + 0.5) * scale),
          drawn,
          drawn,
        );
      })
      .catch(() => {
        if (!cancelled) setFailed(true);
      });
    return () => {
      cancelled = true;
    };
  }, [crosshair, box, zoom]);

  return (
    <canvas
      ref={canvas}
      role="img"
      aria-label={label}
      style={{ width: box, height: box }}
      className={cn(
        "rounded-xl border border-border bg-surface-2",
        failed && "opacity-40",
        className,
      )}
    />
  );
}
