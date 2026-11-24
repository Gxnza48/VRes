import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ipc } from "@/lib/ipc";
import { useEngineStore } from "@/state/engine";
import { useSettingsStore } from "@/state/settings";
import { defaultCrosshair, defaultSettings } from "@/test/fixtures";

import { CrosshairPage } from "./CrosshairPage";
import { parseRendered } from "./render";

vi.mock("@/lib/ipc", () => ({
  inTauri: () => false,
  isIpcError: () => false,
  ipc: {
    crosshairRender: vi.fn().mockResolvedValue(new ArrayBuffer(0)),
    crosshairPresets: vi.fn().mockResolvedValue([]),
    crosshairPreview: vi.fn().mockResolvedValue(undefined),
    settingsUpdate: vi.fn(),
  },
}));

describe("CrosshairPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(ipc.settingsUpdate).mockImplementation((settings) => Promise.resolve(settings));
    vi.mocked(ipc.crosshairPresets).mockResolvedValue([
      { id: "dot", crosshair: { ...defaultCrosshair(), dot: { enabled: true, size: 4, round: true } } },
    ]);
    useEngineStore.setState({ displays: [] });
    useSettingsStore.setState({ settings: defaultSettings() });
  });

  it("prende el crosshair y lo guarda", async () => {
    const user = userEvent.setup();
    render(<CrosshairPage />);
    await user.click(screen.getByRole("button", { name: /Crosshair: Apagado/ }));
    expect(ipc.settingsUpdate).toHaveBeenCalledWith(
      expect.objectContaining({ crosshair: expect.objectContaining({ enabled: true }) }),
    );
  });

  it("aplicar un preset lo guarda como el actual", async () => {
    const user = userEvent.setup();
    render(<CrosshairPage />);
    await user.click(await screen.findByRole("button", { name: /Punto/ }));
    await waitFor(() =>
      expect(ipc.settingsUpdate).toHaveBeenCalledWith(
        expect.objectContaining({
          crosshair: expect.objectContaining({
            current: expect.objectContaining({ dot: { enabled: true, size: 4, round: true } }),
          }),
        }),
      ),
    );
  });

  it("guarda el crosshair actual con un nombre", async () => {
    const user = userEvent.setup();
    render(<CrosshairPage />);
    await user.type(screen.getByLabelText("Nombre"), "Mi mira");
    await user.click(screen.getByRole("button", { name: /Guardar el actual/ }));
    expect(ipc.settingsUpdate).toHaveBeenCalledWith(
      expect.objectContaining({
        crosshair: expect.objectContaining({
          saved: [expect.objectContaining({ name: "Mi mira" })],
        }),
      }),
    );
  });

  it("muestra el aviso de reglas de los juegos", () => {
    render(<CrosshairPage />);
    expect(screen.getByText(/revisa sus reglas/)).toBeInTheDocument();
  });
});

describe("parseRendered", () => {
  it("lee el lado, el centro y los píxeles", () => {
    const buffer = new ArrayBuffer(8 + 4);
    const view = new DataView(buffer);
    view.setUint32(0, 1, true);
    view.setUint32(4, 0, true);
    new Uint8Array(buffer, 8).set([255, 0, 0, 255]);
    const rendered = parseRendered(buffer);
    expect(rendered?.size).toBe(1);
    expect(Array.from(rendered?.rgba ?? [])).toEqual([255, 0, 0, 255]);
  });

  it("rechaza respuestas incompletas", () => {
    expect(parseRendered(new ArrayBuffer(4))).toBeNull();
  });
});
