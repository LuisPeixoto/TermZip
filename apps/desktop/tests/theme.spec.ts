import { describe, expect, it } from "vitest";
import type { Theme, ThemeEntry } from "../src/lib/api";
import {
  ansiPalette,
  chromeTokens,
  contrast,
  cssColor,
  ensureContrast,
  mix,
  parseColor,
  resolveTheme,
  toHex,
  xtermTheme,
} from "../src/lib/theme";
import { appCssTokens, builtIn, builtInThemes } from "./helpers/themes";

const entry = (theme: Theme, builtInFlag = true): ThemeEntry => ({ builtIn: builtInFlag, theme });

describe("color helpers", () => {
  it("reads the spellings a theme file uses and writes them back", () => {
    expect(parseColor("#1e1e2e")).toEqual({ r: 0x1e, g: 0x1e, b: 0x2e, a: 255 });
    expect(parseColor("#fa0")).toEqual({ r: 255, g: 170, b: 0, a: 255 });
    expect(parseColor("#2563EB1f")).toEqual({ r: 0x25, g: 0x63, b: 0xeb, a: 0x1f });
    expect(parseColor("red")).toBeNull();
    expect(parseColor("#12")).toBeNull();
    expect(toHex({ r: 1, g: 2, b: 3, a: 255 })).toBe("#010203");
    expect(toHex({ r: 1, g: 2, b: 3, a: 128 })).toBe("#01020380");
  });

  it("measures contrast like WCAG", () => {
    expect(contrast("#000000", "#ffffff")).toBeCloseTo(21, 5);
    expect(contrast("#ffffff", "#ffffff")).toBeCloseTo(1, 5);
    expect(contrast("#777777", "#ffffff")).toBeCloseTo(4.48, 2);
  });

  it("mixes toward another color", () => {
    expect(mix("#000000", "#ffffff", 0)).toBe("#000000");
    expect(mix("#000000", "#ffffff", 1)).toBe("#ffffff");
    expect(mix("#000000", "#ffffff", 0.5)).toBe("#808080");
  });

  it("writes a transparent color as CSS the way app.css does", () => {
    expect(cssColor("#60a5fa33")).toBe("rgb(96 165 250 / 0.2)");
    expect(cssColor("#2563eb1a")).toBe("rgb(37 99 235 / 0.1)");
    expect(cssColor("#2563eb")).toBe("#2563eb");
  });

  it("moves a color toward the text color only as far as it needs to", () => {
    expect(ensureContrast("#777777", "#ffffff", 3, "#000000")).toBe("#777777");
    const darker = ensureContrast("#aaaaaa", "#ffffff", 4.5, "#000000");
    expect(contrast(darker, "#ffffff")).toBeGreaterThanOrEqual(4.5);
    expect(contrast("#aaaaaa", "#ffffff")).toBeLessThan(4.5);
    // When even the target is not enough, the target is what comes back.
    expect(ensureContrast("#ffffff", "#ffffff", 30, "#000000")).toBe("#000000");
  });
});

describe("the TermZip themes", () => {
  const css = appCssTokens();
  const tokens = [
    "--bg",
    "--panel",
    "--panel-2",
    "--elevated",
    "--hover",
    "--line",
    "--line-strong",
    "--fg",
    "--muted",
    "--faint",
    "--accent",
    "--accent-soft",
    "--ok",
    "--warn",
    "--err",
    "--select",
  ];

  for (const [id, expected] of [
    ["termzip-dark", css.dark],
    ["termzip-light", css.light],
  ] as const) {
    it(`${id} paints the interface exactly as app.css does`, () => {
      const produced = chromeTokens(builtIn(id));
      for (const token of tokens) {
        expect(produced[token].toLowerCase(), `${id} ${token}`).toBe(expected[token].toLowerCase());
      }
    });
  }

  it("keeps white, bright black and bright white readable on the light background", () => {
    const { colors } = builtIn("termzip-light");
    for (const index of [7, 8, 15]) {
      expect(contrast(colors.ansi[index], colors.background), `color ${index}`).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("keeps every light color distinguishable from the background", () => {
    const { colors } = builtIn("termzip-light");
    for (const color of colors.ansi) {
      expect(contrast(color, colors.background)).toBeGreaterThanOrEqual(3);
    }
  });
});

describe("an interface derived from the terminal colors", () => {
  // Without its own chrome, every theme has to give a readable interface from the terminal colors alone.
  for (const theme of builtInThemes()) {
    it(`${theme.id} stays readable without overrides`, () => {
      const plain: Theme = { ...theme, chrome: undefined };
      const t = chromeTokens(plain);

      expect(contrast(t["--fg"], t["--panel"]), "text").toBeGreaterThanOrEqual(4.5);
      for (const surface of ["--panel", "--bg"]) {
        expect(contrast(t["--muted"], t[surface]), `muted on ${surface}`).toBeGreaterThanOrEqual(4.5);
        expect(contrast(t["--faint"], t[surface]), `faint on ${surface}`).toBeGreaterThanOrEqual(3);
        for (const status of ["--accent", "--ok", "--warn", "--err"]) {
          expect(contrast(t[status], t[surface]), `${status} on ${surface}`).toBeGreaterThanOrEqual(3);
        }
      }
    });
  }

  it("lets a theme override any token it wants", () => {
    const theme = builtIn("nord");
    const tokens = chromeTokens({ ...theme, chrome: { panel: "#123456", err: "#ff0000" } });

    expect(tokens["--panel"]).toBe("#123456");
    expect(tokens["--err"]).toBe("#ff0000");
    expect(tokens["--ok"]).not.toBe("#ff0000");
  });

  it("uses a translucent wash for an opaque selection color on the interface", () => {
    const tokens = chromeTokens(builtIn("dracula"));
    expect(tokens["--select"]).toBe("rgb(68 71 90 / 0.45)");
  });
});

describe("the terminal and its output", () => {
  const dracula = builtIn("dracula");

  it("hands xterm the colors of the theme", () => {
    const theme = xtermTheme(dracula);

    expect(theme.background).toBe("#282a36");
    expect(theme.foreground).toBe("#f8f8f2");
    expect(theme.cursor).toBe("#f8f8f2");
    expect(theme.cursorAccent).toBe("#282a36");
    expect(theme.selectionBackground).toBe("#44475a");
    expect(theme.red).toBe("#ff5555");
    expect(theme.brightBlack).toBe("#6272a4");
    expect(theme.brightWhite).toBe("#ffffff");
  });

  it("gives the block output the same 16 colors", () => {
    const palette = ansiPalette(dracula);

    expect(palette.colors).toEqual(dracula.colors.ansi);
    expect(palette.colors).not.toBe(dracula.colors.ansi);
    expect(palette.background).toBe("#282a36");
  });
});

describe("choosing the theme in use", () => {
  const all = builtInThemes().map((theme) => entry(theme));
  const settings = { theme: "dark" as const, darkTheme: "dracula", lightTheme: "catppuccin-latte" };

  it("takes the theme chosen for the current appearance", () => {
    expect(resolveTheme(all, settings)?.id).toBe("dracula");
    expect(resolveTheme(all, { ...settings, theme: "light" })?.id).toBe("catppuccin-latte");
  });

  it("falls back to TermZip's own when the chosen theme is gone", () => {
    expect(resolveTheme(all, { ...settings, darkTheme: "deleted" })?.id).toBe("termzip-dark");
    expect(resolveTheme(all, { theme: "light", darkTheme: "x", lightTheme: "deleted" })?.id).toBe("termzip-light");
  });

  it("does not put a light theme in the dark slot", () => {
    expect(resolveTheme(all, { ...settings, darkTheme: "catppuccin-latte" })?.id).toBe("termzip-dark");
  });

  it("still finds a theme of the right appearance if TermZip's own are missing", () => {
    const withoutDefaults = all.filter((item) => !item.theme.id.startsWith("termzip"));
    expect(resolveTheme(withoutDefaults, { ...settings, darkTheme: "nope" })?.appearance).toBe("dark");
  });

  it("has nothing to offer before the gallery has loaded", () => {
    expect(resolveTheme([], settings)).toBeNull();
  });
});
