import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface Settings {
  theme: "dark" | "light";
  fontSize: number;
  fontFamily: string | null;
  scrollbackLines: number;
  shell: string | null;
  workingDirectory: string | null;
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

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  appInfo: () => invoke<AppInfo>("app_info"),
  detectShells: () => invoke<ShellOption[]>("detect_shells"),
  createSession: (options: {
    rows: number;
    columns: number;
    workingDirectory?: string | null;
    shell?: string | null;
  }) => invoke<SessionInfo>("create_session", options),
  destroySession: (sessionId: number) => invoke<void>("destroy_session", { sessionId }),
  writeSession: (sessionId: number, bytes: Uint8Array) =>
    invoke<void>("write_session", {
      sessionId,
      data: bytesToBase64(bytes),
    }),
  resizeSession: (sessionId: number, rows: number, columns: number) =>
    invoke<void>("resize_session", { sessionId, rows, columns }),
  takeBacklog: (sessionId: number) => invoke<string>("take_backlog", { sessionId }),
  listBlocks: (sessionId: number) => invoke<Block[]>("list_blocks", { sessionId }),
};

export interface PtyData {
  sessionId: number;
  data: string;
}

export interface PtyExit {
  sessionId: number;
  code: number | null;
}

export interface BlocksUpdate {
  sessionId: number;
  blocks: Block[];
}

export function onPtyData(handler: (event: PtyData) => void): Promise<UnlistenFn> {
  return listen<PtyData>("pty-data", (event) => handler(event.payload));
}

export function onPtyExit(handler: (event: PtyExit) => void): Promise<UnlistenFn> {
  return listen<PtyExit>("pty-exit", (event) => handler(event.payload));
}

export function onBlocksUpdate(handler: (event: BlocksUpdate) => void): Promise<UnlistenFn> {
  return listen<BlocksUpdate>("blocks-update", (event) => handler(event.payload));
}

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