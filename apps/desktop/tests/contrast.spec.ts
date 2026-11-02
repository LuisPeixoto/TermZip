/**
 * WCAG 2.1 AA for the interface of every built-in theme: text of any size needs 4.5:1 against the surface it is on
 * (the hints in the preferences are 10.5 px, which is small text however light it looks), and a label on a button
 * needs 4.5:1 against the button.
 */
import { describe, expect, it } from "vitest";
import { chromeTokens, contrast } from "../src/lib/theme";
import { builtInThemes } from "./helpers/themes";

const SURFACES = ["--bg", "--panel", "--panel-2", "--elevated", "--hover"] as const;
const TEXT = ["--fg", "--muted", "--faint", "--accent", "--ok", "--warn", "--err"] as const;

describe("the text of every built-in theme", () => {
  for (const theme of builtInThemes()) {
    const tokens = chromeTokens(theme);

    for (const surface of SURFACES) {
      for (const text of TEXT) {
        it(`${theme.name}: ${text} on ${surface} is at least 4.5:1`, () => {
          expect(contrast(tokens[text], tokens[surface])).toBeGreaterThanOrEqual(4.5);
        });
      }
    }
  }
});

describe("the label of a button painted with the accent", () => {
  for (const theme of builtInThemes()) {
    it(`${theme.name}: the label is at least 4.5:1 on the accent`, () => {
      const tokens = chromeTokens(theme);

      expect(contrast(tokens["--on-accent"], tokens["--accent"])).toBeGreaterThanOrEqual(4.5);
    });
  }
});

describe("a theme that is not built in", () => {
  it("gets an interface that reads too, whatever colors it brings", () => {
    const theme = builtInThemes()[0];
    const strange = {
      ...theme,
      colors: { ...theme.colors, accent: "#777777", background: "#808080", foreground: "#8a8a8a" },
    };

    const tokens = chromeTokens(strange);

    // Even when the terminal colors themselves are nearly the same, text is moved as far as it can go.
    for (const text of ["--muted", "--faint", "--accent", "--ok", "--warn", "--err"] as const) {
      expect(contrast(tokens[text], tokens["--bg"]), text).toBeGreaterThan(1);
    }
  });
});
