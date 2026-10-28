/**
 * Keyboard shortcuts as data: every action has one chord per platform by default, the user's
 * `[keybindings]` replace them, and an event is matched by turning it into a chord.
 *
 * A chord is written `Ctrl+Alt+Shift+Cmd+Key` (see docs/keyboard.md). Letters follow the character
 * that was typed, so a shortcut works on any layout; digits, brackets and the like follow the
 * physical key, since Shift changes what they type.
 */

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
  | "commandPalette"
  | "nextTab"
  | "prevTab"
  | "focusLeft"
  | "focusRight"
  | "focusUp"
  | "focusDown"
  | `tab${TabNumber}`;

/** What the keyboard editor and the command palette list. */
export interface ActionInfo {
  id: AppShortcut;
  label: string;
  group: "General" | "Tabs and panes" | "Terminal";
}

const TABS: TabNumber[] = [1, 2, 3, 4, 5, 6, 7, 8, 9];

export const ACTIONS: readonly ActionInfo[] = [
  { id: "commandPalette", label: "Command palette", group: "General" },
  { id: "preferences", label: "Preferences", group: "General" },
  { id: "blocks", label: "Show or hide the command blocks", group: "General" },
  { id: "newTab", label: "New tab", group: "Tabs and panes" },
  { id: "closePane", label: "Close the pane", group: "Tabs and panes" },
  { id: "splitRight", label: "Split to the right", group: "Tabs and panes" },
  { id: "splitDown", label: "Split downward", group: "Tabs and panes" },
  { id: "nextTab", label: "Next tab", group: "Tabs and panes" },
  { id: "prevTab", label: "Previous tab", group: "Tabs and panes" },
  { id: "focusLeft", label: "Focus the pane on the left", group: "Tabs and panes" },
  { id: "focusRight", label: "Focus the pane on the right", group: "Tabs and panes" },
  { id: "focusUp", label: "Focus the pane above", group: "Tabs and panes" },
  { id: "focusDown", label: "Focus the pane below", group: "Tabs and panes" },
  ...TABS.map((n): ActionInfo => ({ id: `tab${n}`, label: `Go to tab ${n}`, group: "Tabs and panes" })),
  { id: "find", label: "Find in the terminal", group: "Terminal" },
  { id: "restart", label: "Restart the shell", group: "Terminal" },
];

/** One chord per action; `""` is no shortcut at all. */
export type Bindings = Record<AppShortcut, string>;

const isMac = (platform: string) => platform === "macos";

/** The shortcuts TermZip ships with: ⌘ on macOS, Ctrl+Shift elsewhere (plain Ctrl belongs to the shell). */
export function defaultBindings(platform: string): Bindings {
  const mac = isMac(platform);
  const main = mac ? "Cmd+" : "Ctrl+Shift+";
  const tabs = Object.fromEntries(TABS.map((n) => [`tab${n}`, `${main}${n}`]));
  return {
    commandPalette: mac ? "Shift+Cmd+P" : "Ctrl+Shift+P",
    preferences: mac ? "Cmd+Comma" : "Ctrl+Comma",
    restart: `${main}R`,
    blocks: `${main}B`,
    newTab: `${main}T`,
    closePane: `${main}W`,
    splitRight: `${main}D`,
    splitDown: mac ? "Shift+Cmd+D" : "Ctrl+Shift+E",
    find: `${main}F`,
    nextTab: mac ? "Shift+Cmd+BracketRight" : "Ctrl+Tab",
    prevTab: mac ? "Shift+Cmd+BracketLeft" : "Ctrl+Shift+Tab",
    focusLeft: mac ? "Alt+Cmd+Left" : "Ctrl+Alt+Left",
    focusRight: mac ? "Alt+Cmd+Right" : "Ctrl+Alt+Right",
    focusUp: mac ? "Alt+Cmd+Up" : "Ctrl+Alt+Up",
    focusDown: mac ? "Alt+Cmd+Down" : "Ctrl+Alt+Down",
    ...tabs,
  } as Bindings;
}

const ACTION_IDS = new Set<string>(ACTIONS.map((action) => action.id));

/**
 * The defaults with the user's overrides on top. An override for an action that does not exist is
 * ignored; when two actions end up with the same chord the first one in the action list keeps it, so a
 * hand-edited file can never make one key do two things.
 */
export function resolveBindings(overrides: Record<string, string>, platform: string): Bindings {
  const bindings = defaultBindings(platform);
  for (const [action, chord] of Object.entries(overrides)) {
    if (ACTION_IDS.has(action)) bindings[action as AppShortcut] = normalizeChord(chord) ?? "";
  }
  const taken = new Set<string>();
  for (const { id } of ACTIONS) {
    const chord = bindings[id];
    if (!chord) continue;
    if (taken.has(chord)) bindings[id] = "";
    else taken.add(chord);
  }
  return bindings;
}

// --- chords -------------------------------------------------------------------------------------

const MODIFIER_ORDER = ["Ctrl", "Alt", "Shift", "Cmd"] as const;

const NAMED_KEYS: Record<string, string> = {
  ArrowLeft: "Left",
  ArrowRight: "Right",
  ArrowUp: "Up",
  ArrowDown: "Down",
  Tab: "Tab",
  Enter: "Enter",
  " ": "Space",
  Backspace: "Backspace",
  Delete: "Delete",
  Insert: "Insert",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  ",": "Comma",
  ".": "Period",
  "/": "Slash",
  ";": "Semicolon",
  "'": "Quote",
  "[": "BracketLeft",
  "]": "BracketRight",
  "\\": "Backslash",
  "-": "Minus",
  "=": "Equal",
  "`": "Backquote",
};

const MODIFIER_KEYS = new Set(["Control", "Alt", "Shift", "Meta", "AltGraph", "CapsLock", "OS"]);

/** The key of a chord for what was pressed, or `null` for a modifier on its own or a key with no name. */
function keyName(event: Pick<KeyboardEvent, "key"> & { code?: string }): string | null {
  const { key, code = "" } = event;
  if (MODIFIER_KEYS.has(key)) return null;
  // A letter is the character that was typed, so shortcuts hold on Dvorak, AZERTY and the like.
  if (/^[a-z]$/i.test(key)) return key.toUpperCase();
  if (key === ",") return "Comma";
  // Digits and punctuation change with Shift, so they are the physical key.
  const byCode = /^Digit([0-9])$/.exec(code)?.[1] ?? /^Key([A-Z])$/.exec(code)?.[1];
  if (byCode) return byCode;
  const named = code && NAMED_KEYS[codeToKey(code)];
  if (named) return named;
  if (/^F([1-9]|1\d|2[0-4])$/.test(key)) return key;
  if (/^[0-9]$/.test(key)) return key;
  return NAMED_KEYS[key] ?? null;
}

/** `ArrowLeft` stays `ArrowLeft`; `BracketLeft` is the code of `[`. */
function codeToKey(code: string): string {
  const fromCode: Record<string, string> = {
    BracketLeft: "[",
    BracketRight: "]",
    Comma: ",",
    Period: ".",
    Slash: "/",
    Semicolon: ";",
    Quote: "'",
    Backslash: "\\",
    Minus: "-",
    Equal: "=",
    Backquote: "`",
    Space: " ",
  };
  return fromCode[code] ?? code;
}

export type KeyEventLike = Pick<KeyboardEvent, "key" | "metaKey" | "ctrlKey" | "shiftKey" | "altKey"> & {
  code?: string;
};

/** The chord an event is, in canonical spelling; `null` for a modifier pressed alone. */
export function chordOf(event: KeyEventLike): string | null {
  const key = keyName(event);
  if (!key) return null;
  const on = { Ctrl: event.ctrlKey, Alt: event.altKey, Shift: event.shiftKey, Cmd: event.metaKey };
  return [...MODIFIER_ORDER.filter((name) => on[name]), key].join("+");
}

/**
 * A chord in canonical spelling (`cmd+shift+d` becomes `Shift+Cmd+D`), or `null` when it is not
 * one. The rules are those of the backend (`keybindings.rs`), which has the last word.
 */
export function normalizeChord(text: string): string | null {
  const modifiers = new Set<string>();
  let key: string | null = null;
  for (const raw of text.split("+")) {
    const token = raw.trim();
    const modifier = {
      ctrl: "Ctrl",
      control: "Ctrl",
      alt: "Alt",
      option: "Alt",
      opt: "Alt",
      shift: "Shift",
      cmd: "Cmd",
      command: "Cmd",
      meta: "Cmd",
      super: "Cmd",
      win: "Cmd",
    }[token.toLowerCase()];
    if (modifier) {
      if (modifiers.has(modifier)) return null;
      modifiers.add(modifier);
    } else {
      if (key !== null) return null;
      key = canonicalKey(token);
      if (key === null) return null;
    }
  }
  if (key === null) return null;
  const functionKey = /^F\d+$/.test(key);
  if (!functionKey && !modifiers.has("Ctrl") && !modifiers.has("Alt") && !modifiers.has("Cmd")) return null;
  return [...MODIFIER_ORDER.filter((name) => modifiers.has(name)), key].join("+");
}

const KEY_NAMES = new Set([...Object.values(NAMED_KEYS), "Space"]);

function canonicalKey(token: string): string | null {
  if (/^[a-z0-9]$/i.test(token)) return token.toUpperCase();
  const fn = /^f(\d{1,2})$/i.exec(token);
  if (fn) return Number(fn[1]) >= 1 && Number(fn[1]) <= 24 ? `F${Number(fn[1])}` : null;
  for (const name of KEY_NAMES) if (name.toLowerCase() === token.toLowerCase()) return name;
  return null;
}

/** The action a pressed key is bound to, or `null`. */
export function matchChord(event: KeyEventLike, bindings: Bindings): AppShortcut | null {
  const chord = chordOf(event);
  if (!chord) return null;
  for (const { id } of ACTIONS) {
    if (bindings[id] === chord) return id;
  }
  return null;
}

// --- showing a chord ----------------------------------------------------------------------------

const KEY_GLYPHS: Record<string, string> = {
  Left: "←",
  Right: "→",
  Up: "↑",
  Down: "↓",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Semicolon: ";",
  Quote: "'",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Minus: "-",
  Equal: "=",
  Backquote: "`",
};

/** A chord as it is written in tooltips: ⌃⌥⇧⌘ and glyphs on macOS, words joined by + elsewhere. */
export function formatChord(chord: string, platform: string): string {
  if (!chord) return "";
  const parts = chord.split("+");
  const key = parts[parts.length - 1];
  const shown = KEY_GLYPHS[key] ?? key;
  if (isMac(platform)) {
    const glyph: Record<string, string> = { Ctrl: "⌃", Alt: "⌥", Shift: "⇧", Cmd: "⌘" };
    return parts.slice(0, -1).map((name) => glyph[name]).join("") + shown;
  }
  return [...parts.slice(0, -1).map((name) => (name === "Cmd" ? "Win" : name)), shown].join("+");
}

// --- is this chord a good idea ------------------------------------------------------------------

export type Safety = { level: "ok" } | { level: "warn" | "refuse"; message: string };

/** What the terminal itself does with these, so an app shortcut on them would break copy and paste. */
function isClipboardChord(chord: string, platform: string): boolean {
  const base = isMac(platform) ? "Cmd+" : "Ctrl+Shift+";
  return ["C", "V", "A"].some((letter) => chord === `${base}${letter}`);
}

/**
 * Whether a chord may be a shortcut. Refused: one that takes over typing or copy and paste. Warned:
 * one the shell is likely to use itself, because TermZip would then swallow it before the shell sees it.
 */
export function chordSafety(chord: string, platform: string): Safety {
  const normalized = normalizeChord(chord);
  if (!normalized) {
    return { level: "refuse", message: "Use Ctrl, Alt or Cmd with a key, so typing is never taken over." };
  }
  if (isClipboardChord(normalized, platform)) {
    return { level: "refuse", message: "The terminal keeps this one for copy, paste and select all." };
  }
  const parts = normalized.split("+");
  const key = parts[parts.length - 1];
  const mods = new Set(parts.slice(0, -1));
  const letter = /^[A-Z]$/.test(key);
  const mac = isMac(platform);

  if (mods.has("Ctrl") && !mods.has("Cmd") && !mods.has("Alt") && letter && (mac || !mods.has("Shift"))) {
    return { level: "warn", message: `${normalized} is a shell key (readline, Ctrl+C, Ctrl+R…); the shell would no longer receive it.` };
  }
  if (mods.has("Alt") && !mods.has("Ctrl") && !mods.has("Cmd") && (letter || /^[0-9]$/.test(key))) {
    return {
      level: "warn",
      message: mac
        ? "Option with a letter types a character on many keyboards."
        : "Alt with a letter moves by word in many shells; the shell would no longer receive it.",
    };
  }
  return { level: "ok" };
}

/** The action that already has `chord`, other than `except`. */
export function conflictFor(bindings: Bindings, chord: string, except: AppShortcut): ActionInfo | null {
  if (!chord) return null;
  return ACTIONS.find((action) => action.id !== except && bindings[action.id] === chord) ?? null;
}
