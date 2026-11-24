/** Idiomas de la interfaz. El español es el idioma fuente y el de respaldo. */
export const LANGUAGES = ["es", "en", "pt-BR"] as const;

export type Lang = (typeof LANGUAGES)[number];

export const DEFAULT_LANGUAGE: Lang = "es";

const STORAGE_KEY = "vres.language";

/** Convierte una etiqueta BCP 47 (`es-AR`, `pt-BR`, `en-US`…) a un idioma soportado. */
export function toSupported(tag: string | null | undefined): Lang | null {
  if (!tag) return null;
  const lower = tag.toLowerCase();
  if (lower.startsWith("pt")) return "pt-BR";
  if (lower.startsWith("en")) return "en";
  if (lower.startsWith("es")) return "es";
  return null;
}

/** El primer candidato soportado, o español. */
export function pickLanguage(...candidates: Array<string | null | undefined>): Lang {
  for (const candidate of candidates) {
    const lang = toSupported(candidate);
    if (lang) return lang;
  }
  return DEFAULT_LANGUAGE;
}

/** Idioma elegido por el usuario, si lo eligió. */
export function readStoredLanguage(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

export function storeLanguage(lang: Lang): void {
  try {
    localStorage.setItem(STORAGE_KEY, lang);
  } catch {
    // Sin almacenamiento disponible: el idioma vale solo para esta sesión.
  }
}
