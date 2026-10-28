/** Application shortcuts. Chords never collide with what a shell expects: ⌘ on macOS, Ctrl+Shift elsewhere. */
import {
  chordOf,
  defaultBindings,
  formatChord,
  matchChord,
  resolveBindings,
  type AppShortcut,
  type Bindings,
  type KeyEventLike,
  type TabNumber,
} from "./keybindings";

export type { AppShortcut, TabNumber };

/** The user's `[keybindings]` (action → chord); what is left out is the platform's default. */
export type Overrides = Record<string, string>;

const bindingsFor = (platform: string, overrides: Overrides): Bindings =>
  Object.keys(overrides).length === 0 ? defaultBindings(platform) : resolveBindings(overrides, platform);

/** The action an event is bound to, or `null` when it is for the terminal. */
export function matchShortcut(event: KeyEventLike, platform: string, overrides: Overrides = {}): AppShortcut | null {
  // A modifier on its own, or a key with no chord, is never a shortcut.
  if (chordOf(event) === null) return null;
  return matchChord(event, bindingsFor(platform, overrides));
}

/** The 0-based position a `tabN` shortcut points at. */
export function tabPosition(shortcut: AppShortcut): number | null {
  const match = /^tab([1-9])$/.exec(shortcut);
  return match ? Number(match[1]) - 1 : null;
}

/** The shortcut as it is written in tooltips and hints; empty when the action has none. */
export function shortcutLabel(shortcut: AppShortcut, platform: string, overrides: Overrides = {}): string {
  return formatChord(bindingsFor(platform, overrides)[shortcut], platform);
}
