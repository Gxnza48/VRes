import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ipc } from "@/lib/ipc";
import { useEngineStore } from "@/state/engine";
import { useSettingsStore } from "@/state/settings";
import { defaultEnhancers, defaultSettings } from "@/test/fixtures";

import { EnhancersPage } from "./EnhancersPage";

vi.mock("@/lib/ipc", () => ({
  inTauri: () => false,
  isIpcError: () => false,
  ipc: {
    settingsUpdate: vi.fn(),
    hotkeysSuspend: vi.fn().mockResolvedValue(undefined),
    enhancersKeySupported: vi.fn(),
  },
}));

const idle = {
  darkBoostListening: false,
  darkBoost: false,
  zoomListening: false,
  zoom: false,
};

describe("EnhancersPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(ipc.settingsUpdate).mockImplementation((settings) => Promise.resolve(settings));
    vi.mocked(ipc.enhancersKeySupported).mockImplementation((code) =>
      Promise.resolve(code !== "MetaLeft"),
    );
    useEngineStore.setState({ displays: [], enhancers: idle });
    useSettingsStore.setState({ settings: defaultSettings() });
  });

  it("avisa del riesgo y vienen desarmados", () => {
    render(<EnhancersPage />);
    expect(screen.getByText(/algunos juegos y torneos consideran/)).toBeInTheDocument();
    expect(screen.getAllByText("Desarmado")).toHaveLength(2);
  });

  it("arma Dark Boost", async () => {
    const user = userEvent.setup();
    render(<EnhancersPage />);
    await user.click(screen.getAllByRole("button", { name: "Armar" })[0]);
    expect(ipc.settingsUpdate).toHaveBeenCalledWith(
      expect.objectContaining({
        enhancers: expect.objectContaining({
          darkBoost: expect.objectContaining({ armed: true }),
        }),
      }),
    );
  });

  it("graba un botón lateral del mouse para el zoom", async () => {
    const user = userEvent.setup();
    render(<EnhancersPage />);
    await user.click(screen.getAllByRole("button", { name: "Cambiar" })[1]);
    expect(screen.getByText("Aprieta una tecla o un botón del mouse…")).toBeInTheDocument();
    expect(ipc.hotkeysSuspend).toHaveBeenCalledWith(true);
    fireEvent.mouseDown(window, { button: 4 });
    expect(ipc.settingsUpdate).toHaveBeenCalledWith(
      expect.objectContaining({
        enhancers: expect.objectContaining({
          zoom: expect.objectContaining({
            binding: { kind: "mouse", button: "forward" },
          }),
        }),
      }),
    );
  });

  it("no acepta una tecla que no sirve", async () => {
    const user = userEvent.setup();
    render(<EnhancersPage />);
    await user.click(screen.getAllByRole("button", { name: "Cambiar" })[0]);
    fireEvent.keyDown(window, { code: "MetaLeft" });
    expect(
      await screen.findByText("Esa tecla no se puede usar. Prueba con otra."),
    ).toBeInTheDocument();
    expect(ipc.settingsUpdate).not.toHaveBeenCalled();
  });

  it("muestra el estado en vivo y avisa al invertir", () => {
    const enhancers = defaultEnhancers();
    enhancers.zoom.armed = true;
    enhancers.darkBoost.armed = true;
    enhancers.darkBoost.effect.invert = true;
    useSettingsStore.setState({
      settings: { ...defaultSettings(), enhancers },
    });
    useEngineStore.setState({
      enhancers: {
        ...idle,
        zoomListening: true,
        zoom: true,
        darkBoostListening: true,
      },
    });
    render(<EnhancersPage />);
    expect(screen.getByText("Activo")).toBeInTheDocument();
    expect(screen.getByText("Listo")).toBeInTheDocument();
    expect(
      screen.getByText(/puede molestar, o ser peligroso si eres fotosensible/),
    ).toBeInTheDocument();
  });
});
