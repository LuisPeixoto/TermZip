import type { ITheme } from "@xterm/xterm";
import type { AnsiPalette } from "../ansi";

export interface Palette {
  bg: string;
  panel: string;
  panel2: string;
  elevated: string;
  line: string;
  lineStrong: string;
  fg: string;
  muted: string;
  faint: string;
  accent: string;
  ok: string;
  warn: string;
  err: string;
  select: string;
}

declare global {
  interface Document {
    // Populated by app.css custom properties.
  }
}

function readPalette(): Palette {
  const style = getComputedStyle(document.documentElement);
  const token = (name: string, fallback: string): string =>
    style.getPropertyValue(name).trim() || fallback;

  return {
    bg: token("--bg", "#09090b"),
    panel: token("--panel", "#121214"),
    panel2: token("--panel-2", "#18181b"),
    elevated: token("--elevated", "#1f1f23"),
    line: token("--line", "#27272a"),
    lineStrong: token("--line-strong", "#3f3f46"),
    fg: token("--fg", "#fafafa"),
    muted: token("--muted", "#a1a1aa"),
    faint: token("--faint", "#71717a"),
    accent: token("--accent", "#60a5fa"),
    ok: token("--ok", "#34d399"),
    warn: token("--warn", "#fbbf24"),
    err: token("--err", "#f87171"),
    select: token("--select", "rgba(96,165,250,0.2)"),
  };
}

/** The 16 ANSI colors use zinc neutrals and blue-forward primaries. */
const ANSI_DARK = [
  "#3f3f46", // 0 black
  "#f87171", // 1 red
  "#34d399", // 2 green
  "#fbbf24", // 3 yellow
  "#60a5fa", // 4 blue
  "#c084fc", // 5 magenta
  "#22d3ee", // 6 cyan
  "#e4e4e7", // 7 white
  "#71717a", // 8 bright black
  "#fca5a5", // 9 bright red
  "#6ee7b7", // 10 bright green
  "#fde68a", // 11 bright yellow
  "#93c5fd", // 12 bright blue
  "#d8b4fe", // 13 bright magenta
  "#67e8f9", // 14 bright cyan
  "#fafafa", // 15 bright white
];

export const ANSI_LIGHT = [
  "#3f3f46",
  "#dc2626",
  "#059669",
  "#d97706",
  "#2563eb",
  "#7c3aed",
  "#0891b2",
  "#52525b", // white: dark enough to read on the light background
  "#71717a",
  "#b91c1c",
  "#047857",
  "#b45309",
  "#1d4ed8",
  "#6d28d9",
  "#0e7490",
  "#18181b", // bright white
];

export function xtermTheme(): ITheme {
  const palette = readPalette();
  const dark = document.documentElement.dataset.theme === "dark";
  const ansi = dark ? ANSI_DARK : ANSI_LIGHT;
  return {
    background: palette.bg,
    foreground: palette.fg,
    cursor: palette.accent,
    cursorAccent: palette.bg,
    selectionBackground: palette.select,
    selectionForeground: palette.bg,
    black: ansi[0],
    red: ansi[1],
    green: ansi[2],
    yellow: ansi[3],
    blue: ansi[4],
    magenta: ansi[5],
    cyan: ansi[6],
    white: ansi[7],
    brightBlack: ansi[8],
    brightRed: ansi[9],
    brightGreen: ansi[10],
    brightYellow: ansi[11],
    brightBlue: ansi[12],
    brightMagenta: ansi[13],
    brightCyan: ansi[14],
    brightWhite: ansi[15],
  };
}

export function getAnsiPalette(): AnsiPalette {
  const palette = readPalette();
  const dark = document.documentElement.dataset.theme === "dark";
  return {
    foreground: palette.fg,
    background: palette.bg,
    colors: dark ? [...ANSI_DARK] : [...ANSI_LIGHT],
  };
}