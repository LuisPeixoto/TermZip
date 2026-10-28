/**
 * Turns a theme (see docs/themes.md) into what the interface is drawn with: the CSS variables of the
 * window, the xterm.js color theme and the palette of the block output. Pure, so it is tested without
 * a webview.
 */
import type { ITheme } from "@xterm/xterm";
import type { AnsiPalette } from "./ansi";
import type { Settings, Theme, ThemeEntry } from "./api";

export const DEFAULT_DARK_THEME = "termzip-dark";
export const DEFAULT_LIGHT_THEME = "termzip-light";

/** The settings that decide which theme is in use. */
export type ThemeChoice = Pick<Settings, "theme" | "darkTheme" | "lightTheme">;

export interface Rgba {
  r: number;
  g: number;
  b: number;
  a: number;
}

/** `#rgb`, `#rrggbb` or `#rrggbbaa`; anything else is `null`. */
export function parseColor(text: string): Rgba | null {
  const match = /^#([0-9a-f]{3}|[0-9a-f]{6}|[0-9a-f]{8})$/i.exec(text.trim());
  if (!match) return null;
  let digits = match[1];
  if (digits.length === 3) digits = [...digits].map((digit) => digit + digit).join("");
  const byte = (at: number) => Number.parseInt(digits.slice(at, at + 2), 16);
  return { r: byte(0), g: byte(2), b: byte(4), a: digits.length === 8 ? byte(6) : 255 };
}

function hex2(value: number): string {
  return Math.round(Math.min(255, Math.max(0, value))).toString(16).padStart(2, "0");
}

/** `#rrggbb`, with the alpha as `aa` only when the color is not opaque. */
export function toHex(color: Rgba): string {
  const base = `#${hex2(color.r)}${hex2(color.g)}${hex2(color.b)}`;
  return color.a === 255 ? base : base + hex2(color.a);
}

function rgb(text: string): Rgba {
  return parseColor(text) ?? { r: 0, g: 0, b: 0, a: 255 };
}

/** WCAG relative luminance. */
export function luminance(text: string): number {
  const { r, g, b } = rgb(text);
  const channel = (value: number) => {
    const unit = value / 255;
    return unit <= 0.03928 ? unit / 12.92 : ((unit + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

/** WCAG contrast ratio, 1 to 21. */
export function contrast(a: string, b: string): number {
  const [light, dark] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (light + 0.05) / (dark + 0.05);
}

/** `amount` of `over` on top of `base`, as an opaque color. */
export function mix(base: string, over: string, amount: number): string {
  const from = rgb(base);
  const to = rgb(over);
  const part = (a: number, b: number) => a + (b - a) * amount;
  return toHex({ r: part(from.r, to.r), g: part(from.g, to.g), b: part(from.b, to.b), a: 255 });
}

/** A color as CSS: `#rrggbb`, or `rgb(r g b / a)` when it has an alpha. */
export function cssColor(text: string): string {
  const { r, g, b, a } = rgb(text);
  if (a === 255) return toHex({ r, g, b, a });
  return `rgb(${r} ${g} ${b} / ${Number((a / 255).toFixed(2))})`;
}

/** `color` with the given alpha (0..1), as CSS. */
export function withAlpha(text: string, alpha: number): string {
  const { r, g, b } = rgb(text);
  return `rgb(${r} ${g} ${b} / ${alpha})`;
}

/**
 * `color` itself when it reads well enough on `background`, otherwise the nearest color on the way to
 * `toward` that does (or `toward` when even that is not enough).
 */
export function ensureContrast(color: string, background: string, minimum: number, toward: string): string {
  if (contrast(color, background) >= minimum) return color;
  for (let step = 1; step <= 20; step += 1) {
    const candidate = mix(color, toward, step / 20);
    if (contrast(candidate, background) >= minimum) return candidate;
  }
  return toward;
}

/** `ensureContrast` against several backgrounds at once: the color has to read on each of them. */
function readableOn(color: string, backgrounds: string[], minimum: number, toward: string): string {
  return backgrounds.reduce((current, background) => ensureContrast(current, background, minimum, toward), color);
}

/**
 * The CSS variables of the window for a theme. A theme only has to set what it wants different: the
 * rest is derived from its terminal colors, so any imported theme gives a complete interface.
 */
export function chromeTokens(theme: Theme): Record<string, string> {
  const dark = theme.appearance === "dark";
  const { background, foreground, ansi } = theme.colors;
  const own = theme.chrome ?? {};

  const panel = own.panel ?? (dark ? mix(background, foreground, 0.04) : mix(background, "#ffffff", 0.7));
  const panel2 = own.panel_2 ?? mix(background, foreground, dark ? 0.065 : 0.03);
  const elevated = own.elevated ?? (dark ? mix(background, foreground, 0.09) : panel);
  const hover = own.hover ?? mix(background, foreground, dark ? 0.11 : 0.06);
  const line = own.line ?? mix(background, foreground, dark ? 0.125 : 0.1);
  const lineStrong = own.line_strong ?? mix(background, foreground, dark ? 0.22 : 0.17);
  // Text and controls sit on the panels and on the terminal's own background, so they must read on both.
  const surfaces = [panel, background];
  const muted = own.muted ?? readableOn(mix(background, foreground, dark ? 0.64 : 0.74), surfaces, 4.5, foreground);
  const faint = own.faint ?? readableOn(mix(background, foreground, dark ? 0.43 : 0.39), surfaces, 3, foreground);
  // The accent colors controls and focus rings, so it must be seen whatever the theme says.
  const accent = readableOn(theme.colors.accent ?? ansi[4], surfaces, 3, foreground);
  const status = (own: string | undefined, fromAnsi: string) =>
    own ?? readableOn(fromAnsi, surfaces, 3, foreground);
  const selection = parseColor(theme.colors.selection);

  return {
    "--bg": background,
    "--panel": panel,
    "--panel-2": panel2,
    "--elevated": elevated,
    "--hover": hover,
    "--line": line,
    "--line-strong": lineStrong,
    "--fg": foreground,
    "--muted": muted,
    "--faint": faint,
    "--accent": accent,
    "--accent-soft": own.accent_soft ? cssColor(own.accent_soft) : withAlpha(accent, dark ? 0.14 : 0.1),
    "--ok": status(own.ok, ansi[2]),
    "--warn": status(own.warn, ansi[3]),
    "--err": status(own.err, ansi[1]),
    // An opaque selection color is meant for the terminal; as a wash on the interface it must let through.
    "--select": selection && selection.a < 255 ? cssColor(theme.colors.selection) : withAlpha(theme.colors.selection, 0.45),
  };
}

/** Writes the theme into the document: the tokens, and `data-theme` for the dark and light variants. */
export function applyTheme(theme: Theme, root: HTMLElement = document.documentElement): void {
  for (const [name, value] of Object.entries(chromeTokens(theme))) {
    root.style.setProperty(name, value);
  }
  root.dataset.theme = theme.appearance;
}

export function xtermTheme(theme: Theme): ITheme {
  const { colors } = theme;
  const [black, red, green, yellow, blue, magenta, cyan, white] = colors.ansi;
  const [brightBlack, brightRed, brightGreen, brightYellow, brightBlue, brightMagenta, brightCyan, brightWhite] =
    colors.ansi.slice(8);
  return {
    background: colors.background,
    foreground: colors.foreground,
    cursor: colors.cursor,
    cursorAccent: colors.cursor_text ?? colors.background,
    selectionBackground: colors.selection,
    black,
    red,
    green,
    yellow,
    blue,
    magenta,
    cyan,
    white,
    brightBlack,
    brightRed,
    brightGreen,
    brightYellow,
    brightBlue,
    brightMagenta,
    brightCyan,
    brightWhite,
  };
}

/** What the block output is colored with. */
export function ansiPalette(theme: Theme): AnsiPalette {
  return {
    foreground: theme.colors.foreground,
    background: theme.colors.background,
    colors: [...theme.colors.ansi],
  };
}

/**
 * The theme in use: the one chosen for the current appearance. An id that names nothing (a custom
 * theme that was deleted) or a theme of the other appearance falls back to TermZip's own, so the
 * window is never left without colors.
 */
export function resolveTheme(entries: readonly ThemeEntry[], settings: ThemeChoice): Theme | null {
  const dark = settings.theme === "dark";
  const find = (id: string) => entries.find((entry) => entry.theme.id === id)?.theme;
  const chosen = find(dark ? settings.darkTheme : settings.lightTheme);
  if (chosen && chosen.appearance === settings.theme) return chosen;
  return (
    find(dark ? DEFAULT_DARK_THEME : DEFAULT_LIGHT_THEME) ??
    entries.find((entry) => entry.theme.appearance === settings.theme)?.theme ??
    null
  );
}

/** The gallery's two groups, each in the order the backend lists them (TermZip's own first). */
export function groupThemes(entries: readonly ThemeEntry[]): { dark: ThemeEntry[]; light: ThemeEntry[] } {
  return {
    dark: entries.filter((entry) => entry.theme.appearance === "dark"),
    light: entries.filter((entry) => entry.theme.appearance === "light"),
  };
}

/** Who made a theme and under what license, for the line under its name. */
export function describeCredit(theme: Theme): string {
  return [theme.author, theme.license].filter((part): part is string => Boolean(part)).join(" · ");
}
