import { describe, expect, it } from "vitest";

import { resources } from "./index";

function flatten(value: unknown, prefix = ""): string[] {
  if (value && typeof value === "object") {
    return Object.entries(value).flatMap(([key, child]) =>
      flatten(child, prefix ? `${prefix}.${key}` : key),
    );
  }
  return [prefix];
}

describe("traducciones", () => {
  const source = flatten(resources.es.common).sort();

  it.each(["en", "pt-BR"] as const)("%s tiene exactamente las claves del español", (lang) => {
    expect(flatten(resources[lang].common).sort()).toEqual(source);
  });
});
