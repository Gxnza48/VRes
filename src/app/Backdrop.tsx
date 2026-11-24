import { useEffect, useRef, useState } from "react";

import { events } from "@/lib/events";
import { inTauri, ipc } from "@/lib/ipc";
import { useSettingsStore } from "@/state/settings";

/**
 * La imagen o el GIF de fondo elegido en Personalizar. Mientras la ventana no
 * está al frente se muestra quieto (un cuadro del GIF), para no gastar
 * mientras juegas.
 */
export function Backdrop() {
  const background = useSettingsStore((store) => store.settings?.appearance.background ?? null);
  const opacity = useSettingsStore((store) => store.settings?.appearance.backgroundOpacity ?? 25);
  const [url, setUrl] = useState<string | null>(null);
  const [still, setStill] = useState<string | null>(null);
  const [focused, setFocused] = useState(true);
  const image = useRef<HTMLImageElement>(null);
  const stillRef = useRef<string | null>(null);

  const mime = background?.mime;
  const revision = background?.revision;
  useEffect(() => {
    if (!mime || !inTauri()) return;
    let cancelled = false;
    let created: string | null = null;
    ipc
      .customizeBackgroundGet()
      .then((bytes) => {
        if (cancelled || bytes.byteLength === 0) return;
        created = URL.createObjectURL(new Blob([bytes], { type: mime }));
        setUrl(created);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
      if (created) URL.revokeObjectURL(created);
    };
  }, [mime, revision]);

  useEffect(() => {
    if (!inTauri()) return;
    let active = true;
    let stop: (() => void) | undefined;
    void events
      .onFocusChanged((isFocused) => {
        const img = image.current;
        if (!isFocused && img && img.naturalWidth > 0) {
          // Un cuadro quieto en lugar del GIF animado.
          const canvas = document.createElement("canvas");
          canvas.width = img.naturalWidth;
          canvas.height = img.naturalHeight;
          canvas.getContext("2d")?.drawImage(img, 0, 0);
          canvas.toBlob((blob) => {
            if (!blob || !active) return;
            if (stillRef.current) URL.revokeObjectURL(stillRef.current);
            stillRef.current = URL.createObjectURL(blob);
            setStill(stillRef.current);
          });
        }
        setFocused(isFocused);
      })
      .then((unlisten) => {
        if (active) stop = unlisten;
        else unlisten();
      });
    return () => {
      active = false;
      stop?.();
      if (stillRef.current) URL.revokeObjectURL(stillRef.current);
    };
  }, []);

  if (!background || !url) return null;
  return (
    <div
      aria-hidden
      className="pointer-events-none absolute inset-0 z-0 overflow-hidden"
      style={{ opacity: opacity / 100 }}
    >
      {focused || !still ? (
        <img ref={image} src={url} alt="" className="h-full w-full object-cover" />
      ) : (
        <img src={still} alt="" className="h-full w-full object-cover" />
      )}
    </div>
  );
}
