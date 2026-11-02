/**
 * What the command palette lists: every action (with its shortcut), a way to reach each tab, to open a
 * tab with each profile, to switch to each theme, and a few switches. The things to do are handed in,
 * so the list is built and tested without the stores.
 */
import type { Profile, SnippetInfo, TaskInfo, Theme, ThemeEntry } from "./api";
import { ACTIONS, formatChord, type AppShortcut, type Bindings } from "./keybindings";
import { tabPosition } from "./shortcuts";
import { needsForm } from "./snippets";

export interface PaletteEntry {
  /** Stable, so the recently used ones can be remembered. */
  id: string;
  title: string;
  /** The shortcut, or what kind of command this is. */
  detail: string;
  /** The palette stays open after this one is chosen, because it goes on to ask something (a snippet's values). */
  keepsOpen?: boolean;
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
  /** The snippets that apply in the folder of the active pane. */
  snippets: readonly SnippetInfo[];
  /** The web page of the repository the active pane is in, when it has one and git is on. */
  repository: string | null;
  /** The tasks of the project the active pane is in (its scripts, Cargo commands, make targets). */
  tasks: readonly TaskInfo[];
  /** `"snippets"` lists nothing but the snippets (the Insert a snippet shortcut). */
  scope?: "snippets" | null;
  commands: {
    run(action: AppShortcut): void;
    goToTab(position: number): void;
    newTab(profileId: string | null): void;
    useTheme(theme: Theme): void;
    toggleAppearance(): void;
    toggleBlur(): void;
    runPluginCommand(plugin: string, command: string): void;
    /** Types the snippet, or opens the form that asks for its values. */
    useSnippet(snippet: SnippetInfo): void;
    /** Types the command of a task at the prompt; it is never run. */
    typeTask(task: TaskInfo): void;
    /** Opens the page of the repository in the browser. */
    openRepository(url: string): void;
  };
}

/** `github.com` for a repository page; the whole address when it cannot be read. */
function hostOf(url: string): string {
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
}

export function paletteEntries(input: PaletteInput): PaletteEntry[] {
  const { platform, bindings, commands } = input;
  const entries: PaletteEntry[] = [];
  const key = (action: AppShortcut) => formatChord(bindings[action], platform);

  const snippets: PaletteEntry[] = input.snippets.map((snippet) => ({
    id: `snippet:${snippet.id}`,
    title: `Snippet: ${snippet.name}`,
    detail: "snippet",
    keepsOpen: needsForm(snippet),
    run: () => commands.useSnippet(snippet),
  }));
  if (input.scope === "snippets") return snippets;

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

  entries.push(...snippets);

  if (input.repository) {
    const url = input.repository;
    entries.push({
      id: "git:open-repository",
      title: "Open the repository in the browser",
      detail: hostOf(url),
      run: () => commands.openRepository(url),
    });
  }

  for (const task of input.tasks) {
    entries.push({
      id: `task:${task.source}:${task.name}`,
      title: `Task: ${task.command}`,
      detail: task.source,
      run: () => commands.typeTask(task),
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
