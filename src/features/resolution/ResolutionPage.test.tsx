import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { DisplayInfo } from "@/lib/bindings/DisplayInfo";
import type { ResolutionState } from "@/lib/bindings/ResolutionState";
import type { Settings } from "@/lib/bindings/Settings";
import { ipc } from "@/lib/ipc";
import { useEngineStore } from "@/state/engine";
import { useResolutionStore } from "@/state/resolution";
import { useSettingsStore } from "@/state/settings";
import { defaultSettings } from "@/test/fixtures";

import { aspectLabel, stretchLabel } from "./format";
import { ResolutionPage } from "./ResolutionPage";

vi.mock("@/lib/ipc", () => ({
  inTauri: () => false,
  isIpcError: (value: unknown) => typeof value === "object" && value !== null && "code" in value,
  ipc: {
    resolutionApply: vi.fn().mockResolvedValue(null),
    resolutionConfirm: vi.fn().mockResolvedValue(undefined),
    resolutionRevert: vi.fn().mockResolvedValue(undefined),
    resolutionRestore: vi.fn().mockResolvedValue(undefined),
    resolutionToggle: vi.fn().mockResolvedValue(undefined),
    hdrSet: vi.fn().mockResolvedValue(undefined),
    settingsUpdate: vi.fn(),
    hotkeysCheck: vi.fn(),
    hotkeysSuspend: vi.fn().mockResolvedValue(undefined),
  },
}));

const display: DisplayInfo = {
  id: "d-mag",
  gdiName: "\\\\.\\DISPLAY1",
  friendlyName: "MAG 255F E20",
  adapter: { vendor: "nvidia", name: "GPU" },
  bounds: { x: 0, y: 0, width: 1920, height: 1080 },
  primary: true,
  internal: false,
  refreshHz: 200,
  hdr: { supported: true, enabled: false },
};

function state(pending: ResolutionState["pending"] = null): ResolutionState {
  return {
    rev: 1,
    pending,
    displays: [
      {
        displayId: "d-mag",
        current: { width: 1920, height: 1080, refreshHz: 200 },
        saved: { width: 1920, height: 1080, refreshHz: 200 },
        native: { width: 1920, height: 1080, refreshHz: 60 },
        scaling: "identity",
        changed: false,
        modes: [
          { width: 1920, height: 1080, refreshHz: 200 },
          { width: 1440, height: 1080, refreshHz: 200 },
          { width: 1280, height: 960, refreshHz: 60 },
        ],
        presets: [
          {
            width: 1440,
            height: 1080,
            group: "aspect4x3",
            refreshHz: 200,
            available: true,
            stretch: 1.3333,
          },
          {
            width: 1350,
            height: 1080,
            group: "aspect5x4",
            refreshHz: 0,
            available: false,
            stretch: 1.4222,
          },
        ],
      },
    ],
  };
}

const settings: Settings = defaultSettings();

describe("ResolutionPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useEngineStore.setState({ displays: [display] });
    useSettingsStore.setState({ settings });
    useResolutionStore.setState({
      state: state(),
      selectedId: null,
      busy: false,
      error: null,
      lastResolved: null,
    });
  });

  it("muestra el modo actual y los presets", () => {
    render(<ResolutionPage />);
    expect(screen.getByRole("heading", { name: "Resolución" })).toBeInTheDocument();
    expect(screen.getByText("MAG 255F E20")).toBeInTheDocument();
    const unavailable = screen.getByRole("button", { name: /1350 × 1080/ });
    expect(unavailable).toBeDisabled();
    expect(within(unavailable).getByText(/Requiere resolución personalizada/)).toBeInTheDocument();
  });

  it("probar un preset pide el modo estirado por defecto", async () => {
    const user = userEvent.setup();
    render(<ResolutionPage />);
    await user.click(screen.getByRole("button", { name: /1440 × 1080/ }));
    expect(ipc.resolutionApply).toHaveBeenCalledWith("d-mag", {
      width: 1440,
      height: 1080,
      refreshHz: 200,
      scaling: "stretched",
    });
  });

  it("el escalado elegido viaja con el pedido", async () => {
    const user = userEvent.setup();
    render(<ResolutionPage />);
    await user.click(screen.getByRole("radio", { name: "Mantener aspecto" }));
    await user.click(screen.getByRole("button", { name: /1440 × 1080/ }));
    expect(ipc.resolutionApply).toHaveBeenCalledWith(
      "d-mag",
      expect.objectContaining({ scaling: "aspectRatio" }),
    );
  });

  it("un cambio pendiente muestra la cuenta regresiva y se puede conservar", async () => {
    useResolutionStore.setState({
      state: state({
        token: "t-1",
        displayId: "d-mag",
        mode: { width: 1440, height: 1080, refreshHz: 200, scaling: "stretched" },
        expiresInMs: 15_000,
        seconds: 15,
        reason: null,
      }),
    });
    const user = userEvent.setup();
    render(<ResolutionPage />);
    const dialog = screen.getByRole("alertdialog", { name: "¿Conservar esta resolución?" });
    expect(within(dialog).getByText("15")).toBeInTheDocument();
    expect(within(dialog).getByRole("button", { name: "Revertir" })).toHaveFocus();
    await user.click(within(dialog).getByRole("button", { name: "Conservar" }));
    expect(ipc.resolutionConfirm).toHaveBeenCalledWith("t-1");
  });

  it("los errores del motor se muestran traducidos", async () => {
    vi.mocked(ipc.resolutionApply).mockRejectedValueOnce({ code: "modeBusy", message: "busy" });
    const user = userEvent.setup();
    render(<ResolutionPage />);
    await user.click(screen.getByRole("button", { name: /1440 × 1080/ }));
    expect(await screen.findByRole("alert")).toHaveTextContent(/otro cambio de resolución/);
  });
});

describe("formatos", () => {
  it("reconoce las proporciones comunes", () => {
    expect(aspectLabel(1920, 1080)).toBe("16:9");
    expect(aspectLabel(1440, 1080)).toBe("4:3");
    expect(aspectLabel(1280, 1024)).toBe("5:4");
    expect(aspectLabel(2880, 1800)).toBe("16:10");
    expect(aspectLabel(1366, 768)).toBe("16:9");
    expect(aspectLabel(1720, 1080)).toBe("16:10");
    expect(aspectLabel(3440, 1440)).toBe("21:9");
    expect(aspectLabel(1600, 1024, "es")).toBe("1,56:1");
  });

  it("formatea cuánto estira según el idioma", () => {
    expect(stretchLabel(4 / 3, "es")).toBe("×1,33");
    expect(stretchLabel(4 / 3, "en")).toBe("×1.33");
  });
});
