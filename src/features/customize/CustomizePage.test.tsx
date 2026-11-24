import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ipc } from "@/lib/ipc";
import { useSettingsStore } from "@/state/settings";
import { defaultSettings } from "@/test/fixtures";

import { CustomizePage } from "./CustomizePage";

vi.mock("@/lib/ipc", () => ({
  inTauri: () => false,
  isIpcError: () => false,
  ipc: {
    settingsUpdate: vi.fn(),
    customizeBackgroundSet: vi.fn(),
    customizeBackgroundClear: vi.fn(),
  },
}));

describe("CustomizePage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(ipc.settingsUpdate).mockImplementation((settings) => Promise.resolve(settings));
    useSettingsStore.setState({ settings: defaultSettings() });
  });

  it("cambia el tema", async () => {
    const user = userEvent.setup();
    render(<CustomizePage />);
    expect(screen.getByRole("button", { name: /Negro/ })).toHaveAttribute("aria-pressed", "true");
    await user.click(screen.getByRole("button", { name: /Grafito/ }));
    expect(ipc.settingsUpdate).toHaveBeenCalledWith(
      expect.objectContaining({ appearance: expect.objectContaining({ theme: "graphite" }) }),
    );
  });

  it("cambia el color de acento y el tamaño de la interfaz", async () => {
    const user = userEvent.setup();
    render(<CustomizePage />);
    expect(screen.getByRole("button", { name: /Lima/ })).toHaveAttribute("aria-pressed", "true");
    await user.click(screen.getByRole("button", { name: /Violeta/ }));
    expect(ipc.settingsUpdate).toHaveBeenLastCalledWith(
      expect.objectContaining({ appearance: expect.objectContaining({ accent: "violet" }) }),
    );
    await user.click(screen.getByRole("button", { name: /^Grande/ }));
    expect(ipc.settingsUpdate).toHaveBeenLastCalledWith(
      expect.objectContaining({ appearance: expect.objectContaining({ uiScale: 110 }) }),
    );
  });

  it("sube una imagen de fondo", async () => {
    const user = userEvent.setup();
    vi.mocked(ipc.customizeBackgroundSet).mockImplementation(() =>
      Promise.resolve({
        ...defaultSettings(),
        appearance: {
          ...defaultSettings().appearance,
          background: { mime: "image/png", revision: 1 },
        },
      }),
    );
    render(<CustomizePage />);
    const file = new File([new Uint8Array([0x89, 0x50, 0x4e, 0x47])], "fondo.png", {
      type: "image/png",
    });
    await user.upload(screen.getByLabelText("Elegir imagen o GIF"), file);
    expect(ipc.customizeBackgroundSet).toHaveBeenCalledWith(expect.any(Uint8Array));
    expect(await screen.findByRole("button", { name: /Quitar fondo/ })).toBeInTheDocument();
    expect(screen.getByLabelText("Cuánto se ve")).toBeInTheDocument();
    expect(screen.getByLabelText("Desenfoque")).toBeInTheDocument();
  });
});
