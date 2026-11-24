#!/usr/bin/env node
// Prohíbe colores literales en la interfaz fuera de los tokens.
//
// Todo color sale de `src/styles/tokens.css` (o de los primitivos de
// `src/components/ui`). Así, rediseñar VRes es cambiar un archivo.
//
// Uso: npm run check:tokens

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const srcDir = join(root, "src");
const ALLOWED = ["src/styles/", "src/components/ui/", "src/lib/bindings/"];
const EXTENSIONS = [".ts", ".tsx", ".css"];

const PALETTE =
  "slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose";
const RULES = [
  { name: "color hexadecimal", re: /(?<![\w&/])#(?:[0-9a-fA-F]{8}|[0-9a-fA-F]{6}|[0-9a-fA-F]{3,4})\b/ },
  { name: "función de color", re: /\b(?:rgba?|hsla?|oklch|oklab|lab|lch|hwb)\(/ },
  {
    name: "color de la paleta de Tailwind",
    re: new RegExp(
      `\\b(?:bg|text|border|ring|fill|stroke|from|via|to|outline|decoration|shadow|accent|caret|divide|placeholder)-(?:${PALETTE})-\\d{2,3}\\b`,
    ),
  },
  { name: "blanco/negro literal", re: /\b(?:bg|text|border|ring|fill|stroke)-(?:black|white)\b/ },
  { name: "color arbitrario", re: /\b(?:bg|text|border|ring|fill|stroke)-\[(?:#|rgb|hsl|oklch)/ },
];

function walk(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}

let problems = 0;
for (const file of walk(srcDir)) {
  const rel = relative(root, file).split(sep).join("/");
  if (!EXTENSIONS.some((ext) => rel.endsWith(ext))) continue;
  if (ALLOWED.some((dir) => rel.startsWith(dir))) continue;
  readFileSync(file, "utf8")
    .split(/\r?\n/)
    .forEach((line, index) => {
      for (const rule of RULES) {
        const match = line.match(rule.re);
        if (match) {
          console.error(`✗ ${rel}:${index + 1}: ${rule.name} "${match[0]}"`);
          problems++;
        }
      }
    });
}

if (problems > 0) {
  console.error(`\n${problems} color(es) fuera de los tokens. Usa las clases de src/styles/index.css.`);
  process.exit(1);
}
console.log("✓ Sin colores literales fuera de los tokens.");
