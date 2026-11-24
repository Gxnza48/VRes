import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

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
