/// <reference types="vitest/config" />
import { fileURLToPath, URL } from "node:url";
import process from "node:process";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
  },

  // Opciones pensadas para Tauri:
  // 1. no tapar los errores de Rust en la terminal;
  clearScreen: false,
  // 2. Tauri espera un puerto fijo: si está ocupado, fallar;
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    // 3. no vigilar los archivos de Rust.
    watch: { ignored: ["**/src-tauri/**", "**/crates/**"] },
  },

  build: {
    // WebView2 es Chromium moderno.
    target: "es2022",
    rolldownOptions: {
      // Dos ventanas: la principal y el popup rápido de color (F5).
      input: {
        main: fileURLToPath(new URL("./index.html", import.meta.url)),
        quick: fileURLToPath(new URL("./quick.html", import.meta.url)),
      },
    },
  },

  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
    css: false,
  },
});
