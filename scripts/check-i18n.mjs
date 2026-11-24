#!/usr/bin/env node
// Verifica que todos los idiomas tengan exactamente las mismas claves que el
// español (idioma fuente) y que ninguna traducción esté vacía.
//
// Uso: npm run check:i18n

import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const localesDir = join(root, "src", "locales");
const SOURCE = "es";

function flatten(value, prefix = "") {
  if (value && typeof value === "object") {
    return Object.entries(value).flatMap(([key, child]) =>
      flatten(child, prefix ? `${prefix}.${key}` : key),
    );
  }
  return [[prefix, value]];
}

function load(lang, file) {
  return JSON.parse(readFileSync(join(localesDir, lang, file), "utf8"));
}

const languages = readdirSync(localesDir);
const files = readdirSync(join(localesDir, SOURCE)).filter((file) => file.endsWith(".json"));
let problems = 0;

for (const file of files) {
  const source = new Map(flatten(load(SOURCE, file)));
  for (const lang of languages) {
    let entries;
    try {
      entries = new Map(flatten(load(lang, file)));
    } catch (error) {
      console.error(`✗ ${lang}/${file}: no se pudo leer (${error.message})`);
      problems++;
      continue;
    }
    for (const key of source.keys()) {
      if (!entries.has(key)) {
        console.error(`✗ ${lang}/${file}: falta "${key}"`);
        problems++;
      }
    }
    for (const [key, value] of entries) {
      if (!source.has(key)) {
        console.error(`✗ ${lang}/${file}: sobra "${key}" (no existe en ${SOURCE})`);
        problems++;
      }
      if (typeof value === "string" && value.trim() === "") {
        console.error(`✗ ${lang}/${file}: "${key}" está vacía`);
        problems++;
      }
    }
  }
}

if (problems > 0) {
  console.error(`\n${problems} problema(s) en las traducciones.`);
  process.exit(1);
}
console.log(`✓ Traducciones completas: ${languages.join(", ")} (${files.length} archivo/s).`);
