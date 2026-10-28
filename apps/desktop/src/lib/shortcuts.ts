/** Application shortcuts. Chords never collide with what a shell expects: ⌘ on macOS, Ctrl+Shift elsewhere. */

export type TabNumber = 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9;

export type AppShortcut =
  | "preferences"
  | "restart"
  | "blocks"
  | "newTab"
  | "closePane"
  | "splitRight"
  | "splitDown"
  | "find"
  | "nextTab"
  | "prevTab"
  | "focusLeft"
  | "focusRight"
  | "focusUp"
  | "focusDown"
  | `tab${TabNumber}`;

type KeyChord = Pick<KeyboardEvent, "key" | "metaKey" | "ctrlKey" | "shiftKey" | "altKey"> & {
  code?: string;
};

const ARROWS: Record<string, AppShortcut> = {
  ArrowLeft: "focusLeft",
  ArrowRight: "focusRight",
  ArrowUp: "focusUp",
  ArrowDown: "focusDown",
};

/** `Digit3` (or a plain `3`) as a tab number. The key itself is `#` when Shift is held, so the code is used. */
function digit(event: KeyChord): TabNumber | null {
  const fromCode = /^Digit([1-9])$/.exec(event.code ?? "")?.[1];
  const found = fromCode ?? (/^[1-9]$/.test(event.key) ? event.key : undefined);
  return found ? (Number(found) as TabNumber) : null;
}

export function matchShortcut(event: KeyChord, platform: string): AppShortcut | null {
  const mac = platform === "macos";
  const key = event.key.toLowerCase();

  // Preferences uses the plain platform modifier, as in most applications.
  if (key === "," && !event.shiftKey && !event.altKey && (mac ? event.metaKey : event.ctrlKey)) {
    return "preferences";
  }

  // Moving between panes: ⌘⌥ + arrow, or Ctrl+Alt + arrow.
  if (event.altKey) {
    const modifier = mac ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey;
    return modifier && !event.shiftKey ? (ARROWS[event.key] ?? null) : null;
  }

  if (mac) {
    if (!event.metaKey || event.ctrlKey) return null;
    if (event.shiftKey) {
      if (key === "d") return "splitDown";
      if (event.code === "BracketLeft") return "prevTab";
      if (event.code === "BracketRight") return "nextTab";
      return null;
    }
    const tab = digit(event);
    if (tab) return `tab${tab}`;
    return (
      ({ r: "restart", b: "blocks", t: "newTab", w: "closePane", d: "splitRight", f: "find" } as const)[
        key as "r"
      ] ?? null
    );
  }

  // Windows and Linux: plain Ctrl+Tab cycles tabs (the shell never sees it), everything else is Ctrl+Shift.
  if (event.ctrlKey && !event.metaKey && key === "tab") return event.shiftKey ? "prevTab" : "nextTab";
  if (!(event.ctrlKey && event.shiftKey) || event.metaKey) return null;
  const tab = digit(event);
  if (tab) return `tab${tab}`;
  return (
    ({ r: "restart", b: "blocks", t: "newTab", w: "closePane", d: "splitRight", e: "splitDown", f: "find" } as const)[
      key as "r"
    ] ?? null
  );
}

/** The 0-based position a `tabN` shortcut points at. */
export function tabPosition(shortcut: AppShortcut): number | null {
  const match = /^tab([1-9])$/.exec(shortcut);
  return match ? Number(match[1]) - 1 : null;
}

const LETTERS: Partial<Record<AppShortcut, string>> = {
  preferences: ",",
  restart: "R",
  blocks: "B",
  newTab: "T",
  closePane: "W",
  splitRight: "D",
  find: "F",
};

const ARROW_GLYPHS: Partial<Record<AppShortcut, string>> = {
  focusLeft: "←",
  focusRight: "→",
  focusUp: "↑",
  focusDown: "↓",
};

/** The shortcut as it is written in tooltips and hints. */
export function shortcutLabel(shortcut: AppShortcut, platform: string): string {
  const mac = platform === "macos";
  const tab = tabPosition(shortcut);
  if (tab !== null) return mac ? `⌘${tab + 1}` : `Ctrl+Shift+${tab + 1}`;
  const arrow = ARROW_GLYPHS[shortcut];
  if (arrow) return mac ? `⌥⌘${arrow}` : `Ctrl+Alt+${arrow}`;
  switch (shortcut) {
    case "splitDown":
      return mac ? "⇧⌘D" : "Ctrl+Shift+E";
    case "nextTab":
      return mac ? "⇧⌘]" : "Ctrl+Tab";
    case "prevTab":
      return mac ? "⇧⌘[" : "Ctrl+Shift+Tab";
    case "preferences":
      return mac ? "⌘," : "Ctrl+,";
  }
  const letter = LETTERS[shortcut] ?? "";
  return mac ? `⌘${letter}` : `Ctrl+Shift+${letter}`;
}
