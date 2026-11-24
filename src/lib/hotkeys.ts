/** Atajos de teclado: nombres de teclas físicas para mostrar. */

import type { Hotkey } from "@/lib/bindings/Hotkey";

/** Teclas con un símbolo universal. */
const KEY_SYMBOLS: Record<string, string> = {
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  Backquote: "`",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Semicolon: ";",
  Quote: "'",
  Backslash: "\\",
  Comma: ",",
  Period: ".",
  Slash: "/",
  IntlBackslash: "<",
};

/** Teclas cuyo nombre depende del idioma (`resolution.keys.*`). */
const TRANSLATED_KEYS: Record<string, string> = {
  Space: "space",
  PageUp: "pageUp",
  PageDown: "pageDown",
  Insert: "insert",
  Delete: "delete",
  Home: "home",
  End: "end",
};

/** Traduce una clave de i18n. */
export type Translate = (key: string) => string;

/** Nombre corto de una tecla física (`KeyR` → "R", `Numpad5` → "Num 5"). */
export function keyName(code: string, translate: Translate): string {
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  if (code.startsWith("Numpad")) return `Num ${code.slice(6)}`;
  const named = TRANSLATED_KEYS[code];
  if (named) return translate(`resolution.keys.${named}`);
  return KEY_SYMBOLS[code] ?? code;
}

/** "Ctrl + Alt + F8". */
export function hotkeyLabel(hotkey: Hotkey, translate: Translate): string {
  const parts: string[] = [];
  if (hotkey.ctrl) parts.push("Ctrl");
  if (hotkey.alt) parts.push("Alt");
  if (hotkey.shift) parts.push("Shift");
  if (hotkey.win) parts.push("Win");
  parts.push(keyName(hotkey.code, translate));
  return parts.join(" + ");
}

/** Teclas que solo son modificadores: no terminan la grabación de un atajo. */
export const MODIFIER_CODES = new Set([
  "ControlLeft",
  "ControlRight",
  "AltLeft",
  "AltRight",
  "ShiftLeft",
  "ShiftRight",
  "MetaLeft",
  "MetaRight",
  "CapsLock",
  "NumLock",
]);
