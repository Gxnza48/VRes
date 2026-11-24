/** Formatos de la pestaña Resolución. */

import type { DisplayMode } from "@/lib/bindings/DisplayMode";

/** Proporciones conocidas: etiqueta y valor (las ultrapanorámicas reales varían). */
const KNOWN_RATIOS: readonly [string, number][] = [
  ["16:9", 16 / 9],
  ["16:10", 16 / 10],
  ["4:3", 4 / 3],
  ["5:4", 5 / 4],
  ["21:9", 21 / 9],
  ["21:9", 64 / 27],
  ["21:9", 43 / 18],
  ["32:9", 32 / 9],
  ["3:2", 3 / 2],
];

/** `1440 × 1080` → "4:3"; si no es una proporción conocida, "1,56:1". */
export function aspectLabel(width: number, height: number, language = "es"): string {
  const ratio = width / Math.max(height, 1);
  let best: { label: string; diff: number } | null = null;
  for (const [label, value] of KNOWN_RATIOS) {
    const diff = Math.abs(ratio / value - 1);
    if (diff < 0.015 && (!best || diff < best.diff)) best = { label, diff };
  }
  if (best) return best.label;
  const value = new Intl.NumberFormat(language, { maximumFractionDigits: 2 }).format(ratio);
  return `${value}:1`;
}

/** "1920 × 1080". */
export function sizeLabel(width: number, height: number): string {
  return `${width} × ${height}`;
}

/** "1920 × 1080 · 200 Hz". */
export function modeLabel(mode: DisplayMode): string {
  return `${sizeLabel(mode.width, mode.height)} · ${mode.refreshHz} Hz`;
}

/** Cuánto se estira: `1.333` → "×1,33". */
export function stretchLabel(factor: number, language: string): string {
  const value = new Intl.NumberFormat(language, {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(factor);
  return `×${value}`;
}
