/** Lectura de la respuesta de `crosshair_render`. */

export interface Rendered {
  size: number;
  center: number;
  rgba: Uint8ClampedArray;
}

/** Bytes de `crosshair_render`: lado y centro (u32 LE) + RGBA. */
export function parseRendered(buffer: ArrayBuffer): Rendered | null {
  if (buffer.byteLength < 8) return null;
  const header = new DataView(buffer, 0, 8);
  const size = header.getUint32(0, true);
  const center = header.getUint32(4, true);
  const rgba = new Uint8ClampedArray(buffer, 8);
  return rgba.length === size * size * 4 ? { size, center, rgba } : null;
}
