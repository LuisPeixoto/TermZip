/**
 * What the command palette lists: every action (with its shortcut), a way to reach each tab, to open a
 * tab with each profile, to switch to each theme, and a few switches. The things to do are handed in,
 * so the list is built and tested without the stores.
 */
import type { Profile, Theme, ThemeEntry } from "./api";
import { ACTIONS, formatChord, type AppShortcut, type Bindings } from "./keybindings";
import { tabPosition } from "./shortcuts";

export interface PaletteEntry {
  /** Stable, so the recently used ones can be remembered. */
  id: string;
  title: string;
  /** The shortcut, or what kind of command this is. */
  detail: string;
  run: () => void;
}

export interface PaletteInput {
  platform: string;
  bindings: Bindings;
  themes: readonly ThemeEntry[];
  profiles: readonly Profile[];
  /** The titles of the open tabs, in order. */
  tabs: readonly string[];
  /** The theme in use, which is marked in the list. */
  currentThemeId: string | null;
  blurSupported: boolean;
  /** The commands of the plugins that are running. */
  plugins: readonly { plugin: string; pluginName: string; id: string; title: string }[];
  commands: {
    run(action: AppShortcut): void;
    goToTab(position: number): void;
    newTab(profileId: string | null): void;
    useTheme(theme: Theme): void;
    toggleAppearance(): void;
    toggleBlur(): void;
    runPluginCommand(plugin: string, command: string): void;
  };
}

export function paletteEntries(input: PaletteInput): PaletteEntry[] {
  const { platform, bindings, commands } = input;
  const entries: PaletteEntry[] = [];
  const key = (action: AppShortcut) => formatChord(bindings[action], platform);

  // The palette does not list itself, and the tabs are listed by name below instead of "Go to tab 3".
  for (const action of ACTIONS) {
    if (action.id === "commandPalette" || tabPosition(action.id) !== null) continue;
    entries.push({ id: `action:${action.id}`, title: action.label, detail: key(action.id), run: () => commands.run(action.id) });
  }

  for (const entry of input.plugins) {
    entries.push({
      id: `plugin:${entry.plugin}:${entry.id}`,
      title: `${entry.pluginName}: ${entry.title}`,
      detail: "plugin",
      run: () => commands.runPluginCommand(entry.plugin, entry.id),
    });
  }

  input.tabs.slice(0, 9).forEach((title, position) => {
    const action = `tab${position + 1}` as AppShortcut;
    entries.push({
      id: `tab:${position + 1}`,
      title: `Go to tab ${position + 1}: ${title}`,
      detail: key(action),
      run: () => commands.goToTab(position),
    });
  });

  if (input.profiles.length > 0) {
    entries.push({
      id: "new-tab:default",
      title: "New tab: built-in profile",
      detail: "profile",
      run: () => commands.newTab("default"),
    });
    for (const profile of input.profiles) {
      entries.push({
        id: `new-tab:${profile.id}`,
        title: `New tab: ${profile.name}`,
        detail: "profile",
        run: () => commands.newTab(profile.id),
      });
    }
  }

  for (const { theme } of input.themes) {
    entries.push({
      id: `theme:${theme.id}`,
      title: `Theme: ${theme.name}`,
      detail: theme.id === input.currentThemeId ? "in use" : theme.appearance,
      run: () => commands.useTheme(theme),
    });
  }

  entries.push({
    id: "switch:appearance",
    title: "Switch between the dark and light appearance",
    detail: "appearance",
    run: commands.toggleAppearance,
  });
  if (input.blurSupported) {
    entries.push({ id: "switch:blur", title: "Turn the window blur on or off", detail: "window", run: commands.toggleBlur });
  }
  return entries;
}
