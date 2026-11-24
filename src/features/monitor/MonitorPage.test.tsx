import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { DisplayInfo } from "@/lib/bindings/DisplayInfo";
import { ipc } from "@/lib/ipc";
import { useEngineStore } from "@/state/engine";

import { MonitorPage } from "./MonitorPage";

vi.mock("@/lib/ipc", () => ({
  inTauri: () => false,
  isIpcError: () => false,
  ipc: {
    monitorDetails: vi.fn(),
    ddcSet: vi.fn(),
    panelBrightnessSet: vi.fn(),
    hardwareRestoreInitial: vi.fn(),
    displaysSetTrim: vi.fn(),
  },
}));

function display(id: string, internal: boolean): DisplayInfo {
  return {
    id,
    gdiName: `\\\\.\\${id}`,
    friendlyName: internal ? "Panel interno" : "Monitor gamer",
    adapter: { vendor: internal ? "amd" : "nvidia", name: internal ? "Radeon" : "GeForce" },
    bounds: { x: 0, y: 0, width: 1920, height: 1080 },
    primary: internal,
    internal,
    refreshHz: 144,
    hdr: { supported: false, enabled: false },
  };
}

describe("MonitorPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useEngineStore.setState({
      displays: [display("panel", true), display("ext", false)],
      requested: null,
      status: null,
    });
    vi.mocked(ipc.monitorDetails).mockResolvedValue([
      { displayId: "panel", scalePercent: 200, ddc: {}, panelBrightness: 80, hasInitial: false },
      {
        displayId: "ext",
        scalePercent: 100,
        ddc: { brightness: { current: 60, max: 100 }, contrast: { current: 70, max: 100 } },
        panelBrightness: null,
        hasInitial: true,
      },
    ]);
  });

  it("muestra el hardware de cada pantalla", async () => {
    render(<MonitorPage />);
    expect(await screen.findByLabelText("Brillo del panel")).toHaveValue("80");
    expect(screen.getByLabelText("Brillo")).toHaveValue("60");
    expect(screen.getByLabelText("Contraste")).toHaveValue("70");
    expect(screen.getByText("Escala 200 %")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /Volver a los valores de antes de VRes/ }),
    ).toBeInTheDocument();
  });

  it("avisa cuando el monitor no habla DDC/CI", async () => {
    vi.mocked(ipc.monitorDetails).mockResolvedValue([
      { displayId: "panel", scalePercent: null, ddc: {}, panelBrightness: null, hasInitial: false },
      { displayId: "ext", scalePercent: null, ddc: {}, panelBrightness: null, hasInitial: false },
    ]);
    render(<MonitorPage />);
    expect(await screen.findByText(/no responde a DDC\/CI/)).toBeInTheDocument();
    expect(screen.getByText(/no permite cambiar el brillo desde Windows/)).toBeInTheDocument();
  });
});
