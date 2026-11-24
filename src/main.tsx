import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "@fontsource/orbitron/latin-700.css";
import "@fontsource/orbitron/latin-900.css";

import App from "./App";
import { initI18n } from "./i18n";
import "./styles/index.css";

initI18n();

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}
