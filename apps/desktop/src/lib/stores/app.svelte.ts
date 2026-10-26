import type { Block, SessionInfo, Settings } from "../api";
import { api } from "../api";

export interface Toast {
  id: number;
  kind: "info" | "ok" | "warn" | "err";
  message: string;
}

export interface Viewport {
  rows: number;
  columns: number;
}

let toastSequence = 0;

function createAppStore() {
  let settings = $state<Settings>({
    theme: "dark",
    fontSize: 14,
    fontFamily: null,
    scrollbackLines: 10_000,
    shell: null,
    workingDirectory: null,
  });
  let session = $state<SessionInfo | null>(null);
  let exited = $state(false);
  let exitCode = $state<number | null>(null);
  let blocks = $state<Block[]>([]);
  let blocksOpen = $state(true);
  let settingsOpen = $state(false);
  let toasts = $state<Toast[]>([]);
  let viewport = $state<Viewport>({ rows: 0, columns: 0 });

  function applyTheme(theme: Settings["theme"]): void {
    document.documentElement.dataset.theme = theme;
  }

  async function init(): Promise<void> {
    const loaded = await api.getSettings();
    settings = loaded;
    applyTheme(loaded.theme);
  }

  async function toggleTheme(): Promise<void> {
    const current = await api.getSettings();
    const next: Settings = {
      ...current,
      theme: current.theme === "dark" ? "light" : "dark",
    };
    settings = await api.saveSettings(next);
    applyTheme(settings.theme);
  }

  async function updateSettings(patch: Partial<Settings>): Promise<Settings> {
    const current = await api.getSettings();
    settings = await api.saveSettings({ ...current, ...patch });
    applyTheme(settings.theme);
    return settings;
  }

  function toast(kind: Toast["kind"], message: string): void {
    toastSequence += 1;
    const entry: Toast = { id: toastSequence, kind, message };
    toasts = [...toasts.slice(-3), entry];
    window.setTimeout(() => {
      toasts = toasts.filter((candidate) => candidate.id !== entry.id);
    }, 2600);
  }

  function setSession(info: SessionInfo): void {
    session = info;
    exited = false;
    exitCode = null;
    blocks = [];
  }

  function markExited(code: number | null): void {
    exited = true;
    exitCode = code;
  }

  function setViewport(next: Viewport): void {
    viewport = next;
  }

  return {
    get settings() {
      return settings;
    },
    get session() {
      return session;
    },
    get exited() {
      return exited;
    },
    get exitCode() {
      return exitCode;
    },
    get blocks() {
      return blocks;
    },
    get blocksOpen() {
      return blocksOpen;
    },
    get settingsOpen() {
      return settingsOpen;
    },
    get toasts() {
      return toasts;
    },
    get viewport() {
      return viewport;
    },
    init,
    toggleTheme,
    updateSettings,
    toast,
    setSession,
    markExited,
    setBlocks(list: Block[]) {
      blocks = list;
    },
    toggleBlocks() {
      blocksOpen = !blocksOpen;
    },
    toggleSettings() {
      settingsOpen = !settingsOpen;
    },
    closeSettings() {
      settingsOpen = false;
    },
    setViewport,
  };
}

export const app = createAppStore();