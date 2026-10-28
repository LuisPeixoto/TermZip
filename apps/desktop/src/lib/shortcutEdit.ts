/**
 * Changing the shortcuts, as pure functions over the overrides (`action → chord`) that are saved.
 * Only what differs from the platform's defaults is kept, so the file stays small and a default that
 * changes in a later version still reaches everyone who never touched it.
 */
import {
  conflictFor,
  defaultBindings,
  resolveBindings,
  type ActionInfo,
  type AppShortcut,
} from "./keybindings";

export type Overrides = Record<string, string>;

export interface Edit {
  overrides: Overrides;
  /** The action that had the chord and lost it, if any. */
  displaced: ActionInfo | null;
}

/** Keeps the override only when it differs from the default. */
function setOverride(overrides: Overrides, action: AppShortcut, chord: string, platform: string): void {
  if (chord === defaultBindings(platform)[action]) delete overrides[action];
  else overrides[action] = chord;
}

/** Gives `action` the chord. Whoever had it is left without a shortcut, so a key never does two things. */
export function assignChord(overrides: Overrides, platform: string, action: AppShortcut, chord: string): Edit {
  const next = { ...overrides };
  const holder = conflictFor(resolveBindings(overrides, platform), chord, action);
  if (holder) setOverride(next, holder.id, "", platform);
  setOverride(next, action, chord, platform);
  return { overrides: next, displaced: holder };
}

/** Takes the shortcut away from `action`. */
export function clearChord(overrides: Overrides, platform: string, action: AppShortcut): Overrides {
  const next = { ...overrides };
  setOverride(next, action, "", platform);
  return next;
}

/** Puts the default back. If somebody else is using that chord now, they lose it. */
export function resetChord(overrides: Overrides, platform: string, action: AppShortcut): Edit {
  return assignChord(overrides, platform, action, defaultBindings(platform)[action]);
}

/** Whether `action` has a shortcut other than the default one. */
export function isCustomized(overrides: Overrides, platform: string, action: AppShortcut): boolean {
  return resolveBindings(overrides, platform)[action] !== defaultBindings(platform)[action];
}

/** The actions whose label contains every word of `query`, case-insensitively; all of them for an empty query. */
export function filterActions<T extends { label: string }>(actions: readonly T[], query: string): T[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  return actions.filter((action) => words.every((word) => action.label.toLowerCase().includes(word)));
}
