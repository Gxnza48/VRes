import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { DisplayInfo } from "@/lib/bindings/DisplayInfo";
import type { Requested } from "@/lib/bindings/Requested";
import { ipc } from "@/lib/ipc";
import { useEngineStore } from "@/state/engine";

import { DisplayPage } from "./DisplayPage";

vi.mock("@/lib/ipc", () => ({
  inTauri: () => false,
  isIpcError: () => false,
  ipc: {
    displaysIdentify: vi.fn().mockResolvedValue(undefined),
    displaysSetTargets: vi.fn().mockResolvedValue(null),
    lookPreview: vi.fn().mockResolvedValue(undefined),
    lookCommit: vi.fn().mockResolvedValue(null),
    lookReset: vi.fn().mockResolvedValue(null),
    lookSetEnabled: vi.fn().mockResolvedValue(null),
    lookCompare: vi.fn().mockResolvedValue(undefined),
    looksPresets: vi.fn().mockResolvedValue([]),
    crosshairPresets: vi.fn().mockResolvedValue([]),
    gamesList: vi.fn().mockResolvedValue([]),
    shareEncodeLook: vi.fn().mockResolvedValue("VRL1-00000"),
    shareDecode: vi.fn(),
  },
}));

function display(id: string, name: string, x: number): DisplayInfo {
  return {
    id,
    gdiName: `\\\\.\\${id}`,
    friendlyName: name,
    adapter: { vendor: "nvidia", name: "GPU" },
    bounds: { x, y: 0, width: 1920, height: 1080 },
    primary: x === 0,
    internal: false,
    refreshHz: 144,
    hdr: { supported: false, enabled: false },
  };
}

const requested: Requested = {
  colorEnabled: true,
  paused: false,
  compare: false,
  targets: null,
  trims: {},
  exclusiveFullscreen: false,
  look: {
    saturation: 100,
    vibrance: 50,
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
  },
};

describe("DisplayPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useEngineStore.setState({
      ready: true,
      displays: [display("a", "Monitor A", 0), display("b", "Monitor B", 1920)],
      requested,
      status: { rev: 1, layers: [], vibrance: { a: { backend: "nvidia", original: 100 } } },
      notices: [],
    });
  });

  it("muestra cada monitor tildado y permite destildarlo", async () => {
    render(<DisplayPage />);
    expect(screen.getByRole("checkbox", { name: "Monitor A" })).toBeChecked();
    const checkbox = screen.getByRole("checkbox", { name: "Monitor B" });
    expect(checkbox).toBeChecked();
    await userEvent.click(checkbox);
    expect(ipc.displaysSetTargets).toHaveBeenCalledWith(["a"]);
  });

  it("confirma la saturación al soltar el control", () => {
    render(<DisplayPage />);
    const slider = screen.getByRole("slider", { name: "Saturación" });
    fireEvent.change(slider, { target: { value: "180" } });
    fireEvent.pointerUp(slider, { target: { value: "180" } });
    expect(ipc.lookCommit).toHaveBeenCalledWith(
      expect.objectContaining({ saturation: 180 }),
    );
  });

  it("avisa cuando hay monitores sin tildar", () => {
    useEngineStore.setState({ requested: { ...requested, targets: ["a"] } });
    render(<DisplayPage />);
    expect(screen.getByText(/la saturación queda en pausa/)).toBeInTheDocument();
  });

  it("muestra el estado general", () => {
    useEngineStore.setState({
      status: {
        rev: 2,
        layers: [{ layer: "matrix", display: null, status: "ok", reason: null, detail: null }],
        vibrance: {},
      },
    });
    render(<DisplayPage />);
    expect(screen.getByRole("status")).toHaveTextContent("En vivo");
  });
});

describe("vibrance", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useEngineStore.setState({
      ready: true,
      displays: [display("a", "Monitor A", 0), display("b", "Monitor B", 1920)],
      requested,
      status: { rev: 1, layers: [], vibrance: { a: { backend: "nvidia", original: 100 } } },
      notices: [],
    });
  });

  it("muestra qué monitor tiene vibrance del driver y su valor original", () => {
    render(<DisplayPage />);
    expect(screen.getByText("Driver NVIDIA")).toBeInTheDocument();
    expect(screen.getByText("En tu panel: 100")).toBeInTheDocument();
    expect(screen.getByText("Emulada (sin driver)")).toBeInTheDocument();
  });

  it("el control de vibrance confirma el look al soltar", () => {
    render(<DisplayPage />);
    const slider = screen.getByRole("slider", { name: "Vibrance digital" });
    fireEvent.change(slider, { target: { value: "75" } });
    fireEvent.pointerUp(slider, { target: { value: "75" } });
    expect(ipc.lookCommit).toHaveBeenCalledWith(expect.objectContaining({ vibrance: 75 }));
  });
});
