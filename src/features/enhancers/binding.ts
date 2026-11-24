/** Teclas y botones de Dark Boost y Zoom: nombres para mostrar. */
import type { Binding } from "@/lib/bindings/Binding";
import type { MouseButton } from "@/lib/bindings/MouseButton";
import { keyName, type Translate } from "@/lib/hotkeys";

/** `MouseEvent.button` → botón de VRes (el izquierdo, 0, no se puede usar). */
export const MOUSE_BUTTONS: Record<number, MouseButton> = {
  1: "middle",
  2: "right",
  3: "back",
  4: "forward",
};

/** Modificadores solos, que también sirven para "mantener" (`enhancers.keys.*`). */
const LONE_KEYS: Record<string, string> = {
  ShiftLeft: "shiftLeft",
  ShiftRight: "shiftRight",
  ControlLeft: "controlLeft",
  ControlRight: "controlRight",
  AltLeft: "altLeft",
  AltRight: "altRight",
  CapsLock: "capsLock",
};

/** "Botón lateral adelante (Mouse 5)", "Z", "Shift izquierdo"… */
export function bindingLabel(binding: Binding, translate: Translate): string {
  if (binding.kind === "mouse") return translate(`enhancers.buttons.${binding.button}`);
  const lone = LONE_KEYS[binding.code];
  if (lone) return translate(`enhancers.keys.${lone}`);
  return keyName(binding.code, translate);
}

/** `true` si las dos usan la misma tecla o el mismo botón. */
export function sameBinding(a: Binding, b: Binding): boolean {
  if (a.kind === "mouse" && b.kind === "mouse") return a.button === b.button;
  if (a.kind === "key" && b.kind === "key") return a.code === b.code;
  return false;
}
