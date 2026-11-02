import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { TIMEOUT_MS, runBench, type BenchDeps } from "../src/lib/bench";

function setup(plan: { lines: number; command: string } | null = { lines: 1000, command: "seq 1 1000" }) {
  const marks: string[] = [];
  const pane = { cwd: null as string | null, commands: [] as (string | null)[], written: [] as string[] };
  const deps: BenchDeps = {
    plan: async () => plan,
    mark: async (name) => void marks.push(name),
    pane: () => ({
      cwd: pane.cwd,
      commands: pane.commands,
      write: async (text) => void pane.written.push(text),
    }),
    paint: async () => {},
    sleep: (ms) => new Promise((resolve) => setTimeout(resolve, ms)),
  };
  return { deps, marks, pane };
}

describe("bench mode", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("does nothing in a normal run", async () => {
    const { deps, marks, pane } = setup(null);

    await runBench(deps);

    expect(marks).toEqual([]);
    expect(pane.written).toEqual([]);
  });

  it("types the flood once the first prompt is up, and reports each milestone in order", async () => {
    const { deps, marks, pane } = setup({ lines: 1000, command: "seq 1 1000" });

    const run = runBench(deps);
    await vi.advanceTimersByTimeAsync(100);
    expect(marks).toEqual(["mounted", "painted"]);
    expect(pane.written).toEqual([]);

    pane.cwd = "/work";
    await vi.advanceTimersByTimeAsync(100);
    expect(marks).toEqual(["mounted", "painted", "prompt", "flood_start"]);
    expect(pane.written).toEqual(["seq 1 1000\r"]);

    pane.commands.push("ls", "seq 1 1000");
    await vi.advanceTimersByTimeAsync(100);
    await run;
    expect(marks.at(-1)).toBe("done");
  });

  it("types the command the backend chose, for a shell that has no seq", async () => {
    const { deps, marks, pane } = setup({ lines: 3, command: "1..3" });
    pane.cwd = "/work";

    const run = runBench(deps);
    await vi.advanceTimersByTimeAsync(100);
    expect(pane.written).toEqual(["1..3\r"]);
    pane.commands.push("1..3");
    await vi.advanceTimersByTimeAsync(100);
    await run;

    expect(marks.at(-1)).toBe("done");
  });

  it("does not take another command for the flood", async () => {
    const { deps, marks, pane } = setup({ lines: 5, command: "seq 1 5" });
    pane.cwd = "/work";
    pane.commands.push("seq 1 4");

    const run = runBench(deps);
    await vi.advanceTimersByTimeAsync(200);

    expect(marks).not.toContain("done");
    pane.commands.push("seq 1 5");
    await vi.advanceTimersByTimeAsync(100);
    await run;
    expect(marks.at(-1)).toBe("done");
  });

  it("says it failed when the prompt never comes", async () => {
    const { deps, marks } = setup();

    const run = runBench(deps);
    await vi.advanceTimersByTimeAsync(TIMEOUT_MS + 1000);
    await run;

    expect(marks).toEqual(["mounted", "painted", "failed"]);
  });

  it("says it failed when the flood never finishes", async () => {
    const { deps, marks, pane } = setup();
    pane.cwd = "/work";

    const run = runBench(deps);
    await vi.advanceTimersByTimeAsync(TIMEOUT_MS + 1000);
    await run;

    expect(marks.at(-1)).toBe("failed");
    expect(marks).toContain("flood_start");
  });
});
