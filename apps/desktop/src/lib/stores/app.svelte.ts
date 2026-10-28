import type { Settings, ShellOption } from "../api";
import { api } from "../api";
import { focusTerminal } from "../focus";
import { defaultSettings } from "../settings";
import { applyTheme, resolveTheme } from "../theme";
import { currentToasts, toast, type Toast } from "./toasts.svelte";
import { themes } from "./themes.svelte";

export type { Toast };

function createAppStore() {
  let settings = $state<Settings>(defaultSettings());
  let blocksOpen = $state(true);
  let settingsOpen = $state(false);
  let platform = $state("unknown");
  let version = $state("0.2.0");
  let ready = $state(false);
  let shells = $state<ShellOption[]>([]);

  // The theme in use: the one chosen for the current appearance, or TermZip's own when it is gone.
  const theme = $derived(resolveTheme(themes.entries, settings));

  /** Paints the window with the theme in use. */
  function paint(): void {
    if (theme) applyTheme(theme);
  }

  async function init(): Promise<void> {
    const [loaded, info, detected] = await Promise.all([
      api.getSettings(),
      api.appInfo(),
      api.detectShells(),
      themes.load(),
    ]);
    settings = loaded;
    platform = info.platform;
    version = info.version;
    shells = detected;
    document.documentElement.dataset.platform = info.platform;
    paint();
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
      paint();
    } catch (error) {
      toast("err", `Could not change the theme: ${error}`);
    }
  }

  async function updateSettings(patch: Partial<Settings>): Promise<Settings> {
    const current = await api.getSettings();
    settings = await api.saveSettings({ ...current, ...patch });
    paint();
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
    /** The theme in use; `null` only before the gallery has loaded. */
    get theme() {
      return theme;
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
