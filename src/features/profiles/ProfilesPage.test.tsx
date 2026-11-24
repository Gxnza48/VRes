import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ipc } from "@/lib/ipc";
import { useCatalogStore } from "@/state/catalog";
import { useEngineStore } from "@/state/engine";
import { useSettingsStore } from "@/state/settings";
import { defaultSettings } from "@/test/fixtures";

import { ProfilesPage } from "./ProfilesPage";

vi.mock("@/lib/ipc", () => ({
  inTauri: () => false,
  isIpcError: () => false,
  ipc: {
    settingsUpdate: vi.fn(),
    gamesRunningCandidates: vi.fn().mockResolvedValue([{ executable: "Minecraft.exe", title: "Minecraft" }]),
  },
}));

describe("ProfilesPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(ipc.settingsUpdate).mockImplementation((settings) => Promise.resolve(settings));
    useEngineStore.setState({ displays: [], activeProfile: "cs2" });
    useSettingsStore.setState({ settings: defaultSettings() });
    useCatalogStore.setState({
      loaded: true,
      lookPresets: [],
      crosshairPresets: [],
      games: [{ id: "cs2", name: "Counter-Strike 2", executables: ["cs2.exe"], antiCheat: "VAC" }],
    });
  });

  it("prende el cambio automático", async () => {
    const user = userEvent.setup();
    render(<ProfilesPage />);
    await user.click(screen.getByRole("button", { name: /Cambio automático: Apagado/ }));
    expect(ipc.settingsUpdate).toHaveBeenCalledWith(
      expect.objectContaining({ profiles: expect.objectContaining({ autoSwitch: true }) }),
    );
  });

  it("muestra el perfil en uso y el anti-cheat del juego", () => {
    render(<ProfilesPage />);
    expect(screen.getByText("En uso: perfil de Counter-Strike 2.")).toBeInTheDocument();
    expect(screen.getByText(/cs2\.exe · anti-cheat VAC/)).toBeInTheDocument();
  });

  it("activa el perfil de un juego", async () => {
    const user = userEvent.setup();
    render(<ProfilesPage />);
    await user.click(screen.getByRole("checkbox", { name: "Perfil activo" }));
    expect(ipc.settingsUpdate).toHaveBeenCalledWith(
      expect.objectContaining({
        profiles: expect.objectContaining({
          profiles: [expect.objectContaining({ gameId: "cs2", enabled: true })],
        }),
      }),
    );
  });

  it("agrega un juego propio desde los programas abiertos", async () => {
    const user = userEvent.setup();
    render(<ProfilesPage />);
    await screen.findByRole("option", { name: "Minecraft (Minecraft.exe)" });
    await user.selectOptions(screen.getByLabelText("Elige un programa abierto"), "Minecraft.exe");
    await user.click(screen.getByRole("button", { name: /Agregar/ }));
    expect(ipc.settingsUpdate).toHaveBeenCalledWith(
      expect.objectContaining({
        profiles: expect.objectContaining({
          customGames: [{ id: "custom:minecraft.exe", name: "Minecraft", executable: "Minecraft.exe" }],
        }),
      }),
    );
  });
});
