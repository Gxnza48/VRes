import { describe, expect, it } from "vitest";

import { pickLanguage, toSupported } from "./language";

describe("toSupported", () => {
  it("reconoce variantes regionales", () => {
    expect(toSupported("es-AR")).toBe("es");
    expect(toSupported("es-419")).toBe("es");
    expect(toSupported("pt-BR")).toBe("pt-BR");
    expect(toSupported("pt-PT")).toBe("pt-BR");
    expect(toSupported("en-US")).toBe("en");
  });

  it("devuelve null para idiomas no soportados o vacíos", () => {
    expect(toSupported("fr-FR")).toBeNull();
    expect(toSupported("")).toBeNull();
    expect(toSupported(null)).toBeNull();
  });
});

describe("pickLanguage", () => {
  it("usa el primer candidato soportado", () => {
    expect(pickLanguage(null, "fr", "pt-BR", "en")).toBe("pt-BR");
  });

  it("cae en español si no hay ninguno", () => {
    expect(pickLanguage(undefined, "de-DE")).toBe("es");
  });
});
