import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ipc } from "@/lib/ipc";
import { useSettingsStore } from "@/state/settings";
import { defaultSettings } from "@/test/fixtures";

import { BehaviorCard, DiagnosticsCard, ResetCard } from "./SystemCards";

vi.mock("@/lib/ipc", () => ({
  inTauri: () => false,
  isIpcError: () => false,
  ipc: {
    settingsUpdate: vi.fn(),
    diagReport: vi.fn(),
    settingsReset: vi.fn(),
  },
}));

describe("Tarjetas de Más", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(ipc.settingsUpdate).mockImplementation((settings) => Promise.resolve(settings));
    useSettingsStore.setState({ settings: defaultSettings() });
  });

  it("deja elegir que cerrar la ventana cierre VRes", async () => {
    const user = userEvent.setup();
    render(<BehaviorCard />);
    await user.click(screen.getByRole("checkbox", { name: "Seguir en la bandeja" }));
    expect(ipc.settingsUpdate).toHaveBeenCalledWith(
      expect.objectContaining({ behavior: { closeToTray: false } }),
    );
  });

  it("muestra el reporte antes de copiarlo", async () => {
    const user = userEvent.setup();
    vi.mocked(ipc.diagReport).mockResolvedValue('{ "version": "0.8.0" }');
    render(<DiagnosticsCard />);
    await user.click(screen.getByRole("button", { name: /Ver reporte/ }));
    expect(await screen.findByText(/"version": "0.8.0"/)).toBeInTheDocument();
  });

  it("pide confirmación antes de restablecer todo", async () => {
    const user = userEvent.setup();
    vi.mocked(ipc.settingsReset).mockResolvedValue(defaultSettings());
    render(<ResetCard />);
    await user.click(screen.getByRole("button", { name: /Restablecer todo…/ }));
    expect(ipc.settingsReset).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Sí, restablecer todo" }));
    expect(ipc.settingsReset).toHaveBeenCalled();
    expect(await screen.findByText(/VRes quedó como recién instalado/)).toBeInTheDocument();
  });
});
