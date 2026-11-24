/** Lado máximo de un crosshair de imagen, en píxeles. */
export const MAX_IMAGE_SIDE = 256;

/**
 * Convierte una imagen elegida por el usuario en un PNG (`data:` URL) de
 * hasta 256 × 256, conservando la transparencia. Se hace en la interfaz con
 * un `<canvas>`: así cualquier formato que el sistema sepa abrir sirve.
 */
export async function imageToPngDataUrl(file: File): Promise<string> {
  const url = URL.createObjectURL(file);
  try {
    const image = await new Promise<HTMLImageElement>((resolve, reject) => {
      const element = new Image();
      element.onload = () => resolve(element);
      element.onerror = () => reject(new Error("image could not be decoded"));
      element.src = url;
    });
    const scale = Math.min(1, MAX_IMAGE_SIDE / Math.max(image.width, image.height, 1));
    const width = Math.max(1, Math.round(image.width * scale));
    const height = Math.max(1, Math.round(image.height * scale));
    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    const context = canvas.getContext("2d");
    if (!context) throw new Error("canvas unavailable");
    context.drawImage(image, 0, 0, width, height);
    return canvas.toDataURL("image/png");
  } finally {
    URL.revokeObjectURL(url);
  }
}
