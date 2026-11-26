import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ipc } from "@/lib/ipc";
import { useEngineStore } from "@/state/engine";
import { useSettingsStore } from "@/state/settings";
import { defaultCrosshair, defaultSettings } from "@/test/fixtures";

import { SetupPaste } from "./SetupPaste";

vi.mock("@/lib/ipc", () => ({
  inTauri: () => false,
  isIpcError: () => false,
  ipc: {
    shareDecode: vi.fn(),
    lookCommit: vi.fn().mockResolvedValue(null),
    settingsUpdate: vi.fn((settings: unknown) => Promise.resolve(settings)),
    resolutionApply: vi.fn().mockResolvedValue({ token: "t", expiresInMs: 15000 }),
  },
}));

const look = { saturation: 150 } as never;
const mode = { width: 1440, height: 1080, refreshHz: 144, scaling: "stretched" } as const;

describe("SetupPaste", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useSettingsStore.setState({ settings: defaultSettings() });
    useEngineStore.setState({
      displays: [
        {
          id: "d1",
          gdiName: "\\\\.\\DISPLAY1",
          friendlyName: "Monitor",
          adapter: { vendor: "nvidia", name: "GPU" },
          bounds: { x: 0, y: 0, width: 1920, height: 1080 },
          primary: true,
          internal: false,
          refreshHz: 144,
          hdr: { supported: false, enabled: false },
        },
      ],
    });
    const base = { kind: null, look: null, crosshair: null, resolution: null, clamped: false, error: null };
    vi.mocked(ipc.shareDecode).mockImplementation((code: string) =>
      Promise.resolve(
        code.startsWith("VRL1")
          ? { ...base, kind: "look", look }
          : code.startsWith("VRC1")
            ? { ...base, kind: "crosshair", crosshair: defaultCrosshair() }
            : code.startsWith("VRR1")
              ? { ...base, kind: "resolution", resolution: mode }
              : { ...base, error: "badPrefix" },
      ),
    );
  });

  it("aplica colores, mira y resolución de un setup pegado", async () => {
    const user = userEvent.setup();
    render(<SetupPaste />);
    await user.type(screen.getByRole("textbox"), "VRL1-A{enter}VRC1-B{enter}VRR1-C");
    await user.click(screen.getByRole("button", { name: "Aplicar setup" }));
    await waitFor(() => expect(ipc.resolutionApply).toHaveBeenCalledWith("d1", mode));
    expect(ipc.lookCommit).toHaveBeenCalledWith(look);
    expect(ipc.settingsUpdate).toHaveBeenCalledWith(
      expect.objectContaining({ crosshair: expect.objectContaining({ enabled: true }) }),
    );
    expect(await screen.findByText(/Aplicado: colores, mira, resolución/)).toBeInTheDocument();
  });

  it("avisa los códigos que no se pudieron leer", async () => {
    const user = userEvent.setup();
    render(<SetupPaste />);
    await user.type(screen.getByRole("textbox"), "hola");
    await user.click(screen.getByRole("button", { name: "Aplicar setup" }));
    expect(await screen.findByText(/Un código no se pudo leer/)).toBeInTheDocument();
  });
});
