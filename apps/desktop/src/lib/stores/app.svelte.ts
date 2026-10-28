import type { Settings, ShellOption } from "../api";
import { api } from "../api";
import { focusTerminal } from "../focus";
import { defaultSettings } from "../settings";
import { shortcutLabel, type AppShortcut } from "../shortcuts";
import { applyTheme, resolveTheme, type AppearanceChoice } from "../theme";
import { currentToasts, toast, type Toast } from "./toasts.svelte";
import { themes } from "./themes.svelte";

export type { Toast };

function createAppStore() {
  let settings = $state<Settings>(defaultSettings());
  let blocksOpen = $state(true);
  let settingsOpen = $state(false);
  let platform = $state("unknown");
  let version = $state("0.3.0");
  let ready = $state(false);
  let shells = $state<ShellOption[]>([]);

  // While the preferences are open, the choices of the dialog are shown before they are saved.
  let preview = $state.raw<AppearanceChoice | null>(null);
  let blurSupported = $state(false);
  // While a shortcut is being recorded, no shortcut may act: the keys are the answer, not commands.
  let recordingShortcut = $state(false);
  let paletteOpen = $state(false);
  // The commands used last, newest first, so the palette opens on what is likely wanted again.
  let recentActions = $state.raw<string[]>([]);

  const shown = $derived(preview ?? settings);
  // The theme in use: the one chosen for the current appearance, or TermZip's own when it is gone.
  const theme = $derived(resolveTheme(themes.entries, shown));
  const opacity = $derived(shown.backgroundOpacity);
  const translucent = $derived(opacity < 1);

  /** Paints the window with the theme in use. */
  function paint(): void {
    if (theme) applyTheme(theme, opacity);
  }

  // Whatever changes the look (a save, the toggle, a preview, a deleted theme) repaints the window, and
  // the system blurs what shows through only while the window is translucent.
  $effect.root(() => {
    $effect(paint);
    $effect(() => {
      if (!ready) return;
      void api.setWindowEffects(blurSupported && translucent && shown.backgroundBlur).catch(() => {});
    });
  });

  async function init(): Promise<void> {
    const [loaded, info, detected] = await Promise.all([
      api.getSettings(),
      api.appInfo(),
      api.detectShells(),
      themes.load(),
    ]);
    settings = loaded;
    platform = info.platform;
    blurSupported = info.blurSupported;
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
    if (open) paletteOpen = false;
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
    /** Shows these choices as if they were saved; `null` goes back to the saved ones. */
    previewAppearance(choice: AppearanceChoice | null) {
      preview = choice;
    },
    /** The shortcut of an action as the user has it, for tooltips; empty when the action has none. */
    shortcut(action: AppShortcut): string {
      return shortcutLabel(action, platform, settings.keybindings);
    },
    /** ` (⌘R)` for a tooltip, or nothing when the action has no shortcut. */
    hint(action: AppShortcut): string {
      const label = shortcutLabel(action, platform, settings.keybindings);
      return label ? ` (${label})` : "";
    },
    get paletteOpen() {
      return paletteOpen;
    },
    get recentActions() {
      return recentActions;
    },
    rememberAction(id: string) {
      recentActions = [id, ...recentActions.filter((known) => known !== id)].slice(0, 8);
    },
    togglePalette() {
      // The palette and the preferences both take the keyboard; only one at a time.
      if (settingsOpen) return;
      paletteOpen = !paletteOpen;
      if (!paletteOpen) focusTerminal();
    },
    closePalette() {
      paletteOpen = false;
      focusTerminal();
    },
    get recordingShortcut() {
      return recordingShortcut;
    },
    setRecordingShortcut(on: boolean) {
      recordingShortcut = on;
    },
    get opacity() {
      return opacity;
    },
    /** Whether the window lets the desktop show through (the opacity is below 100%). */
    get translucent() {
      return translucent;
    },
    get blurSupported() {
      return blurSupported;
    },
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
