import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  Channel: class {
    onmessage: (message: unknown) => void = () => {};
  },
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import type { Block } from "../src/lib/api";
import { Pane } from "../src/lib/stores/pane.svelte";
import { currentToasts } from "../src/lib/stores/toasts.svelte";

function block(index: number): Block {
  return {
    index,
    command: `cmd ${index}`,
    cwd: null,
    exitCode: 0,
    startedAtMs: null,
    durationMs: null,
    outputChars: 0,
    truncated: false,
    output: null,
  };
}

describe("Pane", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  it("starts empty and remembers where it should start", () => {
    const pane = new Pane("a", "work", "/srv");
    expect(pane.profileId).toBe("work");
    expect(pane.restoreDirectory).toBe("/srv");
    expect(pane.session).toBeNull();
    expect(pane.blocks).toEqual([]);
    expect(pane.exited).toBe(false);
  });

  it("forgets the previous shell when a new one begins", () => {
    const pane = new Pane("a");
    pane.setSession({ id: 3, shell: "zsh", integrated: true });
    pane.addBlock(block(0));
    pane.title = "vim";
    pane.cwd = "/tmp";
    pane.markExited(1);

    pane.begin();

    expect(pane.session).toBeNull();
    expect(pane.blocks).toEqual([]);
    expect(pane.title).toBeNull();
    expect(pane.cwd).toBeNull();
    expect(pane.exited).toBe(false);
    expect(pane.exitCode).toBeNull();
  });

  it("keeps the newest blocks within the cap", () => {
    const pane = new Pane("a");
    for (let index = 0; index < 230; index += 1) pane.addBlock(block(index));
    expect(pane.blocks).toHaveLength(200);
    expect(pane.blocks[0].index).toBe(30);
  });

  it("types into the running shell", async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);
    const pane = new Pane("a");
    pane.setSession({ id: 9, shell: "zsh", integrated: true });

    expect(await pane.write("ls\r")).toBe(true);

    const [command, args] = vi.mocked(invoke).mock.calls[0] as [string, { sessionId: number; data: string }];
    expect(command).toBe("write_session");
    expect(args.sessionId).toBe(9);
    expect(atob(args.data)).toBe("ls\r");
  });

  it("refuses to type before the shell runs or after it exited, and says why", async () => {
    const pane = new Pane("a");
    expect(await pane.write("x")).toBe(false);
    expect(currentToasts().at(-1)?.message).toContain("not running");

    pane.setSession({ id: 1, shell: "zsh", integrated: false });
    pane.markExited(0);
    expect(await pane.write("x")).toBe(false);
    expect(currentToasts().at(-1)?.message).toContain("exited");
    expect(invoke).not.toHaveBeenCalled();
  });

  it("reports a failed write instead of swallowing it", async () => {
    vi.mocked(invoke).mockRejectedValue("pipe closed");
    const pane = new Pane("a");
    pane.setSession({ id: 1, shell: "zsh", integrated: true });

    expect(await pane.write("x")).toBe(false);
    expect(currentToasts().at(-1)?.message).toContain("pipe closed");
  });
});
