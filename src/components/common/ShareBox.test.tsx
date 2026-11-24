import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { ipc } from "@/lib/ipc";
import { defaultCrosshair } from "@/test/fixtures";

import { ShareBox } from "./ShareBox";

vi.mock("@/lib/ipc", () => ({
  inTauri: () => false,
  isIpcError: () => false,
  ipc: { shareDecode: vi.fn() },
}));

describe("ShareBox", () => {
  it("un código dañado muestra el motivo traducido", async () => {
    vi.mocked(ipc.shareDecode).mockResolvedValue({
      kind: "look",
      look: null,
      crosshair: null,
      resolution: null,
      clamped: false,
      error: "badChecksum",
    });
    const onDecoded = vi.fn();
    const user = userEvent.setup();
    render(<ShareBox kind="look" encode={() => Promise.resolve("X")} onDecoded={onDecoded} />);
    await user.type(screen.getByLabelText("Pegar un código"), "VRL1-ABCDE");
    await user.click(screen.getByRole("button", { name: /Aplicar/ }));
    expect(await screen.findByRole("alert")).toHaveTextContent(/se copió mal/);
    expect(onDecoded).not.toHaveBeenCalled();
  });

  it("rechaza un código de otro tipo", async () => {
    vi.mocked(ipc.shareDecode).mockResolvedValue({
      kind: "crosshair",
      look: null,
      crosshair: defaultCrosshair(),
      resolution: null,
      clamped: false,
      error: null,
    });
    const onDecoded = vi.fn();
    const user = userEvent.setup();
    render(<ShareBox kind="look" encode={() => Promise.resolve("X")} onDecoded={onDecoded} />);
    await user.type(screen.getByLabelText("Pegar un código"), "VRC1-ABCDE");
    await user.click(screen.getByRole("button", { name: /Aplicar/ }));
    expect(await screen.findByRole("alert")).toHaveTextContent(/es de otra cosa/);
    expect(onDecoded).not.toHaveBeenCalled();
  });

  it("un código válido llega a quien lo pidió", async () => {
    const decoded = {
      kind: "crosshair" as const,
      look: null,
      crosshair: defaultCrosshair(),
      resolution: null,
      clamped: false,
      error: null,
    };
    vi.mocked(ipc.shareDecode).mockResolvedValue(decoded);
    const onDecoded = vi.fn();
    const user = userEvent.setup();
    render(<ShareBox kind="crosshair" encode={() => Promise.resolve("X")} onDecoded={onDecoded} />);
    await user.type(screen.getByLabelText("Pegar un código"), "vrc1 abcde");
    await user.click(screen.getByRole("button", { name: /Aplicar/ }));
    expect(onDecoded).toHaveBeenCalledWith(decoded);
  });
});
