import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import App from "@/App";
import { TAB_IDS } from "@/app/nav";
import { useAppStore } from "@/state/app";

vi.mock("@/lib/ipc", () => ({
  inTauri: () => false,
  isIpcError: () => false,
  ipc: {
    appBootstrap: vi.fn(),
    appUiReady: vi.fn(),
    appQuit: vi.fn(),
    updateCheck: vi.fn(),
    updateInstall: vi.fn(),
    openExternal: vi.fn(),
  },
}));

describe("Shell", () => {
  beforeEach(() => {
    useAppStore.setState({ tab: "display", bootstrap: null });
  });

  it("muestra las 9 secciones en la barra lateral", () => {
    render(<App />);
    const nav = screen.getByRole("navigation", { name: "Secciones" });
    expect(within(nav).getAllByRole("button")).toHaveLength(TAB_IDS.length);
    expect(within(nav).getByRole("button", { name: "Pantalla" })).toHaveAttribute(
      "aria-current",
      "page",
    );
  });

  it("cambia de sección al hacer clic y dice en qué fase llega", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "Crosshair" }));
    expect(screen.getByRole("heading", { level: 1, name: "Crosshair" })).toBeInTheDocument();
    expect(screen.getByText("Llega en la fase F4")).toBeInTheDocument();
  });

  it("la sección Más permite elegir el idioma", async () => {
    render(<App />);
    await userEvent.click(screen.getByRole("button", { name: "Más" }));
    const select = screen.getByRole("combobox", { name: "Idioma" });
    expect(select).toHaveValue("es");
    expect(screen.getByText("Disponible solo en la app instalada.")).toBeInTheDocument();
  });
});
