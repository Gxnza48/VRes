import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { Look } from "@/lib/bindings/Look";
import { ipc } from "@/lib/ipc";
import { useCatalogStore } from "@/state/catalog";
import { useEngineStore } from "@/state/engine";
import { useSettingsStore } from "@/state/settings";
import { defaultSettings } from "@/test/fixtures";

import { LibraryPage } from "./LibraryPage";

vi.mock("@/lib/ipc", () => ({
  inTauri: () => false,
  isIpcError: () => false,
  ipc: {
    lookCommit: vi.fn().mockResolvedValue(null),
    settingsUpdate: vi.fn(),
    shareEncodeLook: vi.fn().mockResolvedValue("VRL1-00000"),
    shareDecode: vi.fn(),
    crosshairRender: vi.fn().mockResolvedValue(new ArrayBuffer(0)),
  },
}));

const vivid: Look = {
  saturation: 140,
  vibrance: 60,
  tone: {
    brightness: 100,
    contrast: 100,
    gamma: 1,
    temperature: 0,
    highlights: 0,
    shadows: 0,
    blacks: 0,
    whites: 0,
    shadowTint: 0,
    highlightTint: 0,
    gain: [1, 1, 1],
  },
  color: { luma: "rec709", hueShiftDeg: 0, hueShiftAmount: 100, darkenHueDeg: 0, darkenAmount: 0 },
  vision: { mode: "off", strength: 100 },
  eyeCare: { warmth: 0, dim: 0 },
};

describe("LibraryPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(ipc.settingsUpdate).mockImplementation((settings) => Promise.resolve(settings));
    useEngineStore.setState({ displays: [], requested: null });
    useSettingsStore.setState({ settings: defaultSettings() });
    useCatalogStore.setState({
      loaded: true,
      lookPresets: [{ id: "vivid", game: null, look: vivid }],
      crosshairPresets: [],
      games: [{ id: "cs2", name: "Counter-Strike 2" }],
    });
  });

  it("aplica un look incluido", async () => {
    const user = userEvent.setup();
    render(<LibraryPage />);
    const row = screen.getByText("Vívido").closest("li");
    expect(row).not.toBeNull();
    await user.click(within(row as HTMLElement).getByRole("button", { name: /Aplicar/ }));
    expect(ipc.lookCommit).toHaveBeenCalledWith(vivid);
  });

  it("copia un look incluido a los míos", async () => {
    const user = userEvent.setup();
    render(<LibraryPage />);
    await user.click(screen.getByRole("button", { name: /Copiar a los míos/ }));
    expect(ipc.settingsUpdate).toHaveBeenCalledWith(
      expect.objectContaining({
        library: { looks: [expect.objectContaining({ name: "Vívido", look: vivid })] },
      }),
    );
  });

  it("la búsqueda filtra", async () => {
    const user = userEvent.setup();
    render(<LibraryPage />);
    await user.type(screen.getByLabelText("Buscar"), "nada que ver");
    expect(screen.getByText("Nada coincide con la búsqueda.")).toBeInTheDocument();
  });
});
