import { describe, expect, it } from "vitest";
import { ansiToSpans, DEFAULT_PALETTE, type AnsiSpan } from "../src/lib/ansi";

function plain(spans: AnsiSpan[]): string {
  return spans.map((span) => span.text).join("");
}

describe("ansiToSpans", () => {
  it("keeps plain text and newlines", () => {
    expect(plain(ansiToSpans("hello\nworld"))).toBe("hello\nworld");
  });

  it("applies basic colors and resets", () => {
    const spans = ansiToSpans("\x1b[31mred\x1b[0m plain", DEFAULT_PALETTE);
    expect(spans[0].text).toBe("red");
    expect(spans[0].style.fg).toBe(DEFAULT_PALETTE.colors[1]);
    expect(spans.at(-1)?.text).toBe(" plain");
    expect(spans.at(-1)?.style.fg).toBeUndefined();
  });

  it("handles bold, dim, italic, underline and inverse", () => {
    const spans = ansiToSpans("\x1b[1;2;3;4;7mx", DEFAULT_PALETTE);
    expect(spans[0].style).toMatchObject({
      bold: true,
      dim: true,
      italic: true,
      underline: true,
      inverse: true,
    });
  });

  it("handles 256-color and truecolor foregrounds", () => {
    const indexed = ansiToSpans("\x1b[38;5;196mx", DEFAULT_PALETTE);
    expect(indexed[0].style.fg).toMatch(/^rgb\(/);
    const truecolor = ansiToSpans("\x1b[38;2;1;2;3mx", DEFAULT_PALETTE);
    expect(truecolor[0].style.fg).toBe("rgb(1, 2, 3)");
  });

  it("backs up over the previous character", () => {
    expect(plain(ansiToSpans("abc\b\bZ"))).toBe("aZ");
  });

  it("turns carriage returns into new lines", () => {
    expect(plain(ansiToSpans("one\rtwo"))).toBe("one\ntwo");
  });

  it("drops unknown escape sequences", () => {
    expect(plain(ansiToSpans("\x1b]0;title\x07\x1b[2Khello"))).toBe("hello");
  });

  it("expands tabs to the next 8-column boundary", () => {
    const spans = ansiToSpans("ab\tc");
    expect(plain(spans)).toBe("ab      c");
  });
});