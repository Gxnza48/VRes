import i18n from "i18next";
import { initReactI18next } from "react-i18next";

import en from "@/locales/en/common.json";
import es from "@/locales/es/common.json";
import ptBR from "@/locales/pt-BR/common.json";

import {
  DEFAULT_LANGUAGE,
  type Lang,
  pickLanguage,
  readStoredLanguage,
  storeLanguage,
} from "./language";

export const resources = {
  es: { common: es },
  en: { common: en },
  "pt-BR": { common: ptBR },
} as const;

/** Inicia i18n. Sin idioma explícito, usa el guardado o el del sistema. */
export function initI18n(lang?: Lang) {
  const initial = lang ?? pickLanguage(readStoredLanguage(), navigator.language);
  if (i18n.isInitialized) {
    void i18n.changeLanguage(initial);
  } else {
    void i18n.use(initReactI18next).init({
      resources,
      lng: initial,
      fallbackLng: DEFAULT_LANGUAGE,
      defaultNS: "common",
      ns: ["common"],
      initAsync: false,
      interpolation: { escapeValue: false },
    });
  }
  document.documentElement.lang = initial;
  return i18n;
}

/** Cambia y recuerda el idioma de la interfaz. */
export async function setLanguage(lang: Lang): Promise<void> {
  storeLanguage(lang);
  await i18n.changeLanguage(lang);
  document.documentElement.lang = lang;
}

export { i18n };
