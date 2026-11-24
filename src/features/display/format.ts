/** Formatos de los valores de la pestaña Pantalla, según el idioma. */

/** `130` → "130 %" (o "130%", según el idioma). */
export function formatPercent(value: number, language: string): string {
  return new Intl.NumberFormat(language, { style: "percent", maximumFractionDigits: 0 }).format(
    value / 100,
  );
}

/** `1.2` → "1,20" (o "1.20", según el idioma). */
export function formatDecimal(value: number, language: string): string {
  return new Intl.NumberFormat(language, {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(value);
}

/** `12` → "+12", `-12` → "−12", `0` → "0". */
export function formatSigned(value: number): string {
  const rounded = Math.round(value);
  if (rounded > 0) return `+${rounded}`;
  if (rounded < 0) return `−${Math.abs(rounded)}`;
  return "0";
}
