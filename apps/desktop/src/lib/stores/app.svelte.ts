import type { Settings, ShellOption } from "../api";
import { api } from "../api";
import { focusTerminal } from "../focus";
import { currentToasts, toast, type Toast } from "./toasts.svelte";

export type { Toast };

function createAppStore() {
  let settings = $state<Settings>({
    theme: "dark",
    fontSize: 14,
    fontFamily: null,
    scrollbackLines: 10_000,
    shell: null,
    workingDirectory: null,
    restoreSession: true,
    defaultProfile: null,
    profiles: [],
  });
  let blocksOpen = $state(true);
  let settingsOpen = $state(false);
  let platform = $state("unknown");
  let version = $state("0.1.0");
  let ready = $state(false);
  let shells = $state<ShellOption[]>([]);

  function applyTheme(theme: Settings["theme"]): void {
    document.documentElement.dataset.theme = theme;
  }

  async function init(): Promise<void> {
    const [loaded, info, detected] = await Promise.all([
      api.getSettings(),
      api.appInfo(),
      api.detectShells(),
    ]);
    settings = loaded;
    platform = info.platform;
    version = info.version;
    shells = detected;
    document.documentElement.dataset.platform = info.platform;
    applyTheme(loaded.theme);
    ready = true;
  }

  async function toggleTheme(): Promise<void> {
    try {
      const current = await api.getSettings();
      const next: Settings = {
        ...current,
        theme: current.theme === "dark" ? "light" : "dark",
      };
      settings = await api.saveSettings(next);
      applyTheme(settings.theme);
    } catch (error) {
      toast("err", `Could not change the theme: ${error}`);
    }
  }

  async function updateSettings(patch: Partial<Settings>): Promise<Settings> {
    const current = await api.getSettings();
    settings = await api.saveSettings({ ...current, ...patch });
    applyTheme(settings.theme);
    return settings;
  }

  function setSettingsOpen(open: boolean): void {
    settingsOpen = open;
    // The modal held the keyboard; give it back to the terminal once the modal is gone.
    if (!open) focusTerminal();
  }

  return {
    get settings() {
      return settings;
    },
    get blocksOpen() {
      return blocksOpen;
    },
    get settingsOpen() {
      return settingsOpen;
    },
    get toasts() {
      return currentToasts();
    },
    get platform() {
      return platform;
    },
    get version() {
      return version;
    },
    get ready() {
      return ready;
    },
    get shells() {
      return shells;
    },
    init,
    toggleTheme,
    updateSettings,
    toast,
    toggleBlocks() {
      blocksOpen = !blocksOpen;
    },
    toggleSettings() {
      setSettingsOpen(!settingsOpen);
    },
    closeSettings() {
      setSettingsOpen(false);
    },
  };
}

export const app = createAppStore();
