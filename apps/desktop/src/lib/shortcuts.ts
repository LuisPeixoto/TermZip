/** Application shortcuts. Chords never collide with what a shell expects: ⌘ on macOS, Ctrl+Shift elsewhere. */

export type AppShortcut = "preferences" | "restart" | "blocks";

type KeyChord = Pick<KeyboardEvent, "key" | "metaKey" | "ctrlKey" | "shiftKey" | "altKey">;

export function matchShortcut(event: KeyChord, platform: string): AppShortcut | null {
  if (event.altKey) return null;
  const mac = platform === "macos";
  const key = event.key.toLowerCase();

  // Preferences uses the plain platform modifier, as in most applications.
  if (key === "," && !event.shiftKey && (mac ? event.metaKey : event.ctrlKey)) {
    return "preferences";
  }

  const chord = mac
    ? event.metaKey && !event.ctrlKey && !event.shiftKey
    : event.ctrlKey && event.shiftKey && !event.metaKey;
  if (!chord) return null;
  if (key === "r") return "restart";
  if (key === "b") return "blocks";
  return null;
}

const KEYS: Record<AppShortcut, string> = { preferences: ",", restart: "R", blocks: "B" };

/** The shortcut as it is written in tooltips and hints. */
export function shortcutLabel(shortcut: AppShortcut, platform: string): string {
  const key = KEYS[shortcut];
  if (platform === "macos") return `⌘${key}`;
  return shortcut === "preferences" ? `Ctrl+${key}` : `Ctrl+Shift+${key}`;
}
