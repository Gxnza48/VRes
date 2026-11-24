import "@testing-library/jest-dom/vitest";

import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

import { initI18n } from "@/i18n";

// Los tests corren siempre en español, el idioma fuente.
initI18n("es");

afterEach(() => {
  cleanup();
});
