#!/usr/bin/env node
// Verifica que la versión sea la misma en el workspace de Cargo, en
// package.json y (si se pasa) en el tag de git que dispara el release.
//
// Uso: npm run check:version            → compara Cargo y package.json
//      node scripts/check-version.mjs v0.1.0 → además, contra el tag

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (path) => readFileSync(join(root, path), "utf8");

const cargo = read("Cargo.toml");
const section = cargo.split(/^\[workspace\.package\]\s*$/m)[1]?.split(/^\[/m)[0] ?? "";
const cargoVersion = section.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const packageVersion = JSON.parse(read("package.json")).version;
const tauriVersion = JSON.parse(read("src-tauri/tauri.conf.json")).version;
const tag = process.argv[2];

const found = [
  ["Cargo.toml [workspace.package]", cargoVersion],
  ["package.json", packageVersion],
];
if (tauriVersion !== undefined) found.push(["src-tauri/tauri.conf.json", tauriVersion]);
if (tag) found.push([`tag ${tag}`, tag.replace(/^v/, "")]);

const versions = new Set(found.map(([, version]) => version));
if (!cargoVersion || versions.size !== 1) {
  console.error("✗ Las versiones no coinciden:");
  for (const [where, version] of found) console.error(`  ${where}: ${version ?? "(no encontrada)"}`);
  process.exit(1);
}
console.log(`✓ Versión ${cargoVersion} en ${found.map(([where]) => where).join(", ")}.`);
