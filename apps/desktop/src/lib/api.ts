import { Channel, invoke } from "@tauri-apps/api/core";

export interface Profile {
  id: string;
  name: string;
  shell: string | null;
  workingDirectory: string | null;
  env: Record<string, string>;
}

export interface Settings {
  theme: "dark" | "light";
  fontSize: number;
  fontFamily: string | null;
  scrollbackLines: number;
  shell: string | null;
  workingDirectory: string | null;
  restoreSession: boolean;
  defaultProfile: string | null;
  profiles: Profile[];
}

export interface SessionInfo {
  id: number;
  shell: string;
  integrated: boolean;
}

export interface ShellOption {
  path: string;
  label: string;
  default: boolean;
  integrated: boolean;
}

export interface AppInfo {
  version: string;
  platform: string;
  configPath: string;
}

export interface Block {
  index: number;
  command: string | null;
  cwd: string | null;
  exitCode: number | null;
  startedAtMs: number | null;
  durationMs: number | null;
  outputChars: number;
  truncated: boolean;
  output: string | null;
}

/** Everything one shell session reports, in order, on its own channel. */
export type SessionEvent =
  | { type: "data"; data: string }
  | { type: "exit"; code: number | null }
  | { type: "block"; block: Block }
  | { type: "cwd"; path: string };

export type SplitDirection = "row" | "column";

export type PaneNode =
  | { kind: "leaf"; profile: string | null; cwd: string | null }
  | { kind: "split"; direction: SplitDirection; ratio: number; first: PaneNode; second: PaneNode };

export interface WorkspaceTab {
  title: string | null;
  activePane: number;
  root: PaneNode;
}

/** The layout reopened on launch: structure only, never terminal output. */
export interface Workspace {
  version: number;
  activeTab: number;
  tabs: WorkspaceTab[];
}

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  appInfo: () => invoke<AppInfo>("app_info"),
  detectShells: () => invoke<ShellOption[]>("detect_shells"),
  /** Events can arrive before the returned promise settles; `onEvent` must not need the id. */
  createSession: (
    options: {
      rows: number;
      columns: number;
      /** The profile to start with; the backend falls back to the default profile. */
      profileId?: string | null;
      /** Where the pane was when the layout was saved; used only while the folder still exists. */
      restoreDirectory?: string | null;
    },
    onEvent: (event: SessionEvent) => void,
  ) => {
    const channel = new Channel<SessionEvent>();
    channel.onmessage = onEvent;
    return invoke<SessionInfo>("create_session", { ...options, onEvent: channel });
  },
  destroySession: (sessionId: number) => invoke<void>("destroy_session", { sessionId }),
  writeSession: (sessionId: number, bytes: Uint8Array) =>
    invoke<void>("write_session", {
      sessionId,
      data: bytesToBase64(bytes),
    }),
  resizeSession: (sessionId: number, rows: number, columns: number) =>
    invoke<void>("resize_session", { sessionId, rows, columns }),
  /** Tells the backend how much output the terminal has finished drawing (flow control). */
  ackSession: (sessionId: number, bytes: number) =>
    invoke<void>("ack_session", { sessionId, bytes }),
  listBlocks: (sessionId: number) => invoke<Block[]>("list_blocks", { sessionId }),
  /** Opens a URL or a file path that was clicked in the terminal; the backend decides what is allowed. */
  openLink: (target: string, cwd: string | null) => invoke<void>("open_link", { target, cwd }),
  loadWorkspace: () => invoke<Workspace | null>("load_workspace"),
  saveWorkspace: (workspace: Workspace) => invoke<void>("save_workspace", { workspace }),
};

export function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  const chunk = 0x8000;
  for (let index = 0; index < bytes.length; index += chunk) {
    binary += String.fromCharCode(...bytes.subarray(index, index + chunk));
  }
  return btoa(binary);
}

export function base64ToBytes(encoded: string): Uint8Array {
  const binary = atob(encoded);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}