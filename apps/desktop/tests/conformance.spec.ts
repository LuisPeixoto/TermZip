/**
 * What TermZip relies on the terminal emulator for, checked against xterm.js with no window: cursor movement and
 * erasing, colors, the alternate screen, scroll regions, wide and combining characters, links, the modes a program
 * switches on (bracketed paste, mouse), the cursor shape, titles, and the shell-integration markers passing through
 * without being drawn. It is a guard for an upgrade of xterm: when one of these changes, a test here says which.
 */
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { describe, expect, it } from "vitest";
import { Terminal } from "@xterm/headless";

const ESC = "\x1b";

function terminal(cols = 20, rows = 6): Terminal {
  return new Terminal({ cols, rows, allowProposedApi: true, scrollback: 100 });
}

/** Writes and waits until xterm has parsed it. */
const write = (term: Terminal, data: string) => new Promise<void>((resolve) => term.write(data, resolve));

/** A row of the visible screen (the buffer also holds the scrollback above it). */
const line = (term: Terminal, row: number) =>
  term.buffer.active.getLine(term.buffer.active.viewportY + row)?.translateToString(true) ?? "";
const screen = (term: Terminal) => Array.from({ length: term.rows }, (_, row) => line(term, row));

describe("the version", () => {
  it("is the same for the terminal and for the headless copy these tests use", () => {
    const require = createRequire(import.meta.url);
    const version = (name: string) =>
      (JSON.parse(readFileSync(require.resolve(`${name}/package.json`), "utf8")) as { version: string }).version;

    expect(version("@xterm/headless")).toBe(version("@xterm/xterm"));
  });
});

describe("the cursor and erasing", () => {
  it("moves to a row and column (CUP, 1-based)", async () => {
    const term = terminal();
    await write(term, `${ESC}[3;5HX`);

    expect(line(term, 2)).toBe("    X");
  });

  it("moves relative to where it is, and stops at the edges", async () => {
    const term = terminal();
    await write(term, `${ESC}[3;3H${ESC}[2AA${ESC}[100CB${ESC}[100DC`);

    // C at the left edge, A where the cursor went up to, B at the right edge.
    expect(line(term, 0)).toBe(`C A${" ".repeat(16)}B`);
    expect(term.buffer.active.cursorY).toBe(0);
  });

  it("erases the whole screen (ED 2), a line (EL 2) and to the end of a line (EL 0)", async () => {
    const term = terminal();
    await write(term, "aaaa\r\nbbbb\r\ncccc");

    await write(term, `${ESC}[2;3H${ESC}[K`);
    expect(screen(term).slice(0, 3)).toEqual(["aaaa", "bb", "cccc"]);

    await write(term, `${ESC}[3;1H${ESC}[2K`);
    expect(line(term, 2)).toBe("");

    await write(term, `${ESC}[2J`);
    expect(screen(term).every((text) => text === "")).toBe(true);
  });

  it("wraps at the end of a line and scrolls at the bottom", async () => {
    const term = terminal(5, 3);
    await write(term, "abcdefgh\r\n2\r\n3\r\n4");

    expect(screen(term)).toEqual(["2", "3", "4"]);
    expect(term.buffer.active.baseY).toBeGreaterThan(0);
  });
});

describe("colors and styles", () => {
  const cell = (term: Terminal, x = 0, y = 0) => term.buffer.active.getLine(y)!.getCell(x)!;

  it("reads bold, italic, underline and inverse", async () => {
    const term = terminal();
    await write(term, `${ESC}[1;3;4;7mx`);

    const x = cell(term);
    expect([x.isBold(), x.isItalic(), x.isUnderline(), x.isInverse()].map(Boolean)).toEqual([true, true, true, true]);
  });

  it("reads the 16 colors, the 256-color palette and truecolor, for text and background", async () => {
    const term = terminal();
    await write(term, `${ESC}[31ma${ESC}[38;5;196mb${ESC}[38;2;10;20;30mc${ESC}[48;2;1;2;3md${ESC}[0me`);

    expect(cell(term, 0).isFgPalette()).toBeTruthy();
    expect(cell(term, 0).getFgColor()).toBe(1);
    expect(cell(term, 1).getFgColor()).toBe(196);
    expect(cell(term, 2).isFgRGB()).toBeTruthy();
    expect(cell(term, 2).getFgColor()).toBe(0x0a141e);
    expect(cell(term, 3).isBgRGB()).toBeTruthy();
    expect(cell(term, 3).getBgColor()).toBe(0x010203);
    expect(cell(term, 4).isFgDefault()).toBe(true);
  });
});

describe("the alternate screen and scroll regions", () => {
  it("switches to the alternate screen (1049) and back, keeping what was there", async () => {
    const term = terminal();
    await write(term, "shell prompt");

    await write(term, `${ESC}[?1049h${ESC}[2J${ESC}[Hvim`);
    expect(term.buffer.active.type).toBe("alternate");
    expect(line(term, 0)).toBe("vim");

    await write(term, `${ESC}[?1049l`);
    expect(term.buffer.active.type).toBe("normal");
    expect(line(term, 0)).toBe("shell prompt");
  });

  it("scrolls only inside a scroll region (DECSTBM)", async () => {
    const term = terminal(10, 5);
    await write(term, "0\r\n1\r\n2\r\n3\r\n4");

    await write(term, `${ESC}[2;4r${ESC}[4;1H\r\nX`);

    // Rows 1 and 5 stay; 2 to 4 scrolled up by one.
    expect(screen(term)).toEqual(["0", "2", "3", "X", "4"]);
  });
});

describe("characters of different widths", () => {
  it("gives a wide character two cells", async () => {
    const term = terminal();
    await write(term, "漢a");
    const row = term.buffer.active.getLine(0)!;

    expect(row.getCell(0)!.getWidth()).toBe(2);
    expect(row.getCell(1)!.getWidth()).toBe(0);
    expect(row.getCell(2)!.getChars()).toBe("a");
  });

  it("keeps a combining mark in the cell of its letter", async () => {
    const term = terminal();
    await write(term, "éx");
    const row = term.buffer.active.getLine(0)!;

    expect(row.getCell(0)!.getChars()).toBe("é");
    expect(row.getCell(1)!.getChars()).toBe("x");
  });

  it("keeps a character outside the basic plane whole", async () => {
    const term = terminal();
    await write(term, "𝒜b");

    expect(term.buffer.active.getLine(0)!.getCell(0)!.getChars()).toBe("𝒜");
  });
});

describe("what a program switches on", () => {
  it("turns bracketed paste (2004) on and off", async () => {
    const term = terminal();
    expect(term.modes.bracketedPasteMode).toBe(false);

    await write(term, `${ESC}[?2004h`);
    expect(term.modes.bracketedPasteMode).toBe(true);
    await write(term, `${ESC}[?2004l`);
    expect(term.modes.bracketedPasteMode).toBe(false);
  });

  it("follows the mouse tracking modes", async () => {
    const term = terminal();
    for (const [mode, name] of [
      [9, "x10"],
      [1000, "vt200"],
      [1002, "drag"],
      [1003, "any"],
    ] as const) {
      await write(term, `${ESC}[?${mode}h`);
      expect(term.modes.mouseTrackingMode, `mode ${mode}`).toBe(name);
      await write(term, `${ESC}[?${mode}l`);
      expect(term.modes.mouseTrackingMode, `mode ${mode} off`).toBe("none");
    }
  });

  it("follows application cursor keys, wraparound and focus reporting", async () => {
    const term = terminal();
    await write(term, `${ESC}[?1h${ESC}[?1004h`);

    expect(term.modes.applicationCursorKeysMode).toBe(true);
    expect(term.modes.sendFocusMode).toBe(true);
    await write(term, `${ESC}[?7l`);
    expect(term.modes.wraparoundMode).toBe(false);
  });

  it("changes the cursor shape (DECSCUSR)", async () => {
    const term = terminal();

    await write(term, `${ESC}[6 q`);
    expect([term.options.cursorStyle, term.options.cursorBlink]).toEqual(["bar", false]);
    await write(term, `${ESC}[3 q`);
    expect([term.options.cursorStyle, term.options.cursorBlink]).toEqual(["underline", true]);
    await write(term, `${ESC}[2 q`);
    expect([term.options.cursorStyle, term.options.cursorBlink]).toEqual(["block", false]);
  });
});

describe("titles, links and the markers of shell integration", () => {
  it("reports a title set with OSC 0 and OSC 2", async () => {
    const term = terminal();
    const titles: string[] = [];
    term.onTitleChange((title) => titles.push(title));

    await write(term, `${ESC}]0;first\x07${ESC}]2;second${ESC}\\`);

    expect(titles).toEqual(["first", "second"]);
  });

  it("delivers a hyperlink (OSC 8) to a handler, with its address", async () => {
    const term = terminal();
    const seen: string[] = [];
    term.parser.registerOscHandler(8, (data) => {
      seen.push(data);
      return true;
    });

    await write(term, `${ESC}]8;;https://example.com\x07link${ESC}]8;;\x07`);

    expect(seen).toEqual([";https://example.com", ";"]);
    expect(line(term, 0)).toBe("link");
  });

  it("does not draw the markers TermZip reads (OSC 133 and OSC 1337)", async () => {
    const term = terminal(40, 3);

    await write(
      term,
      `${ESC}]133;A\x07$ ${ESC}]133;B\x07${ESC}]1337;CommandB64=bHM=\x07${ESC}]133;C\x07out\r\n${ESC}]133;D;0\x07${ESC}]1337;CurrentDir=/work\x07`,
    );

    expect(line(term, 0)).toBe("$ out");
  });

  it("keeps the scrollback it was told to keep", async () => {
    const term = new Terminal({ cols: 10, rows: 3, scrollback: 50, allowProposedApi: true });
    await write(term, Array.from({ length: 500 }, (_, n) => `row${n}`).join("\r\n"));

    expect(term.buffer.active.length).toBe(3 + 50);
    expect(line(term, 0).startsWith("row")).toBe(true);
  });

  it("copes with a flood of output and with garbage", async () => {
    const term = terminal(80, 24);
    await write(term, Array.from({ length: 20_000 }, (_, n) => `${n}`).join("\r\n"));
    await write(term, `${ESC}[999999999;999999999H${ESC}[?99999h${ESC}]${"x".repeat(100_000)}\x07\x00\x07${ESC}`);

    expect(term.buffer.active.cursorY).toBeLessThan(24);
    expect(term.buffer.active.cursorX).toBeLessThan(80);
  });
});
