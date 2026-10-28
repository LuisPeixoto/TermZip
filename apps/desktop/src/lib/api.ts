import { Channel, invoke } from "@tauri-apps/api/core";

export type CursorStyle = "block" | "bar" | "underline";

export interface Profile {
  id: string;
  name: string;
  shell: string | null;
  workingDirectory: string | null;
  /** The profile's own look for its terminal; the app's when left out. */
  theme?: string | null;
  fontSize?: number | null;
  fontFamily?: string | null;
  cursorStyle?: CursorStyle | null;
  env: Record<string, string>;
}

export interface Settings {
  /** Whether the window is dark or light; each appearance has its own theme below. */
  theme: "dark" | "light";
  darkTheme: string;
  lightTheme: string;
  cursorStyle: CursorStyle;
  cursorBlink: boolean;
  lineHeight: number;
  letterSpacing: number;
  backgroundOpacity: number;
  backgroundBlur: boolean;
  fontSize: number;
  fontFamily: string | null;
  scrollbackLines: number;
  shell: string | null;
  workingDirectory: string | null;
  restoreSession: boolean;
  defaultProfile: string | null;
  /** `action: "Shift+Cmd+E"`; an empty chord unbinds the action. */
  keybindings: Record<string, string>;
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

/** Colors are `#rrggbb`, or `#rrggbbaa` where a transparency matters. */
export interface ThemeColors {
  background: string;
  foreground: string;
  cursor: string;
  cursor_text?: string;
  selection: string;
  accent?: string;
  /** The 16 ANSI colors, black to bright white. */
  ansi: string[];
}

/** Interface colors a theme sets itself; whatever is left out is derived from the terminal colors. */
export interface ThemeChrome {
  panel?: string;
  panel_2?: string;
  elevated?: string;
  hover?: string;
  line?: string;
  line_strong?: string;
  muted?: string;
  faint?: string;
  accent_soft?: string;
  ok?: string;
  warn?: string;
  err?: string;
}

/** A theme exactly as its file spells it (snake_case keys), see docs/themes.md. */
export interface Theme {
  id: string;
  name: string;
  appearance: "dark" | "light";
  author?: string;
  license?: string;
  source?: string;
  colors: ThemeColors;
  chrome?: ThemeChrome;
}

export interface ThemeEntry {
  /** Built-in themes cannot be deleted. */
  builtIn: boolean;
  theme: Theme;
}

export interface ThemeList {
  themes: ThemeEntry[];
  /** One sentence for every custom theme file that was left out. */
  problems: string[];
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
  listThemes: () => invoke<ThemeList>("list_themes"),
  /** Opens a file dialog and keeps the converted theme; `null` when the dialog is cancelled. */
  importTheme: () => invoke<ThemeEntry | null>("import_theme"),
  /** Opens a save dialog and writes the theme in TermZip's format; `false` when it is cancelled. */
  exportTheme: (id: string) => invoke<boolean>("export_theme", { id }),
  deleteTheme: (id: string) => invoke<void>("delete_theme", { id }),
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