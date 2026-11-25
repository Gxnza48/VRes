import type { CSSProperties, ReactNode } from "react";

import { cn } from "@/lib/utils";

export interface SliderRange {
  min: number;
  max: number;
  default: number;
  step: number;
}

interface ParamSliderProps {
  id: string;
  label: string;
  value: number;
  range: SliderRange;
  /** Texto del valor (por ejemplo "130 %"). */
  display: string;
  /** Extremos de la escala (por ejemplo "frío" y "cálido"). */
  scale?: [ReactNode, ReactNode];
  disabled?: boolean;
  /** Título de ayuda (doble clic para volver al neutro). */
  resetTitle: string;
  /** Mientras se arrastra (vista previa). */
  onChange: (value: number) => void;
  /** Al soltar, con teclado o con doble clic (se guarda). */
  onCommit: (value: number) => void;
  /** Oculta la etiqueta y el valor (cuando la tarjeta ya los muestra). */
  hideHeader?: boolean;
  className?: string;
}

/** Control deslizante de un parámetro, con vista previa y confirmación. */
export function ParamSlider({
  id,
  label,
  value,
  range,
  display,
  scale,
  disabled,
  resetTitle,
  onChange,
  onCommit,
  hideHeader,
  className,
}: ParamSliderProps) {
  const commitFrom = (target: EventTarget) => onCommit(Number((target as HTMLInputElement).value));
  const span = range.max - range.min;
  const fill = span > 0 ? ((Math.min(Math.max(value, range.min), range.max) - range.min) / span) * 100 : 0;

  return (
    <div className={cn("flex flex-col gap-1.5", className)}>
      <div
        className={cn(
          "flex items-baseline justify-between gap-3 text-sm",
          hideHeader && "sr-only",
        )}
      >
        <label htmlFor={id} className="text-fg">
          {label}
        </label>
        <span className="tabular-nums text-muted">{display}</span>
      </div>
      <input
        id={id}
        type="range"
        min={range.min}
        max={range.max}
        step={range.step}
        value={value}
        disabled={disabled}
        title={resetTitle}
        style={{ "--fill": `${fill}%` } as CSSProperties}
        onChange={(event) => onChange(Number(event.target.value))}
        onPointerUp={(event) => commitFrom(event.target)}
        onKeyUp={(event) => commitFrom(event.target)}
        onDoubleClick={() => {
          onChange(range.default);
          onCommit(range.default);
        }}
        className="w-full disabled:opacity-50"
      />
      {scale && (
        <div className="flex justify-between text-[11px] text-muted">
          <span>{scale[0]}</span>
          <span>{scale[1]}</span>
        </div>
      )}
    </div>
  );
}
