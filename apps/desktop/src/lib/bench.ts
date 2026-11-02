/**
 * Bench mode: a scripted run that types a flood into the first terminal and reports when each milestone happened,
 * so the app can be measured the same way every time. It only runs when the backend says `TERMZIP_BENCH` is set; in
 * a normal run it asks once and goes away.
 */

export interface BenchDeps {
  plan(): Promise<{ lines: number; command: string } | null>;
  mark(name: string): Promise<void>;
  /** The first terminal: the folder it reported (set at the first prompt), the commands it finished, and typing. */
  pane(): { cwd: string | null; commands: (string | null)[]; write(text: string): Promise<unknown> };
  /** Resolves after the next paint. */
  paint(): Promise<void>;
  sleep(ms: number): Promise<void>;
}

/** Gives up after this long: a run that is stuck must end and say so, not hang a script. */
export const TIMEOUT_MS = 120_000;
const POLL_MS = 20;

async function waitFor(deps: BenchDeps, ready: () => boolean): Promise<boolean> {
  for (let waited = 0; waited < TIMEOUT_MS; waited += POLL_MS) {
    if (ready()) return true;
    await deps.sleep(POLL_MS);
  }
  return false;
}

export async function runBench(deps: BenchDeps): Promise<void> {
  const plan = await deps.plan();
  if (!plan) return;
  await deps.mark("mounted");
  await deps.paint();
  await deps.mark("painted");

  // The folder is reported by the shell's prompt hook, so the first one means the first prompt is up.
  if (!(await waitFor(deps, () => deps.pane().cwd !== null))) return void (await deps.mark("failed"));
  await deps.mark("prompt");

  const { command } = plan;
  await deps.mark("flood_start");
  await deps.pane().write(`${command}\r`);
  const finished = await waitFor(deps, () => deps.pane().commands.includes(command));
  await deps.mark(finished ? "done" : "failed");
}
