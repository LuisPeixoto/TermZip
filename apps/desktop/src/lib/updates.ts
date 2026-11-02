/**
 * Updates, only when asked (see docs/updates.md): what the page shows at each step, and the optional check at startup.
 * The network use itself is in the backend; nothing here reaches the network.
 */
import type { UpdateInfo } from "./api";

export type UpdateState =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "current" }
  | { kind: "available"; info: UpdateInfo }
  | { kind: "installing" }
  | { kind: "error"; message: string };

/** What a check found, as a state of the screen. */
export const found = (info: UpdateInfo | null): UpdateState => (info ? { kind: "available", info } : { kind: "current" });

export const failed = (error: unknown): UpdateState => ({ kind: "error", message: String(error) });

/** One line about an update that is available. */
export const summary = (info: UpdateInfo): string =>
  `TermZip ${info.version} is available. You have ${info.currentVersion}${info.date ? `; released ${info.date}` : ""}.`;

export interface StartupDeps {
  /** The setting. Off by default. */
  enabled: boolean;
  check(): Promise<UpdateInfo | null>;
  notify(text: string): void;
}

/**
 * At startup, when the person turned it on: look once, and only say something when there is an update. A failure is
 * silent (no connection is normal and not worth a message at startup), and nothing is installed.
 */
export async function checkAtStartup(deps: StartupDeps): Promise<void> {
  if (!deps.enabled) return;
  try {
    const info = await deps.check();
    if (info) deps.notify(`${summary(info)} Install it from Preferences → General.`);
  } catch {
    // Offline, or the server is down: try again next time.
  }
}
