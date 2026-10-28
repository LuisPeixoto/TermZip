import { describe, expect, it } from "vitest";
import { cellsToLine, columnsOf, findPathLinks, isOpenGesture } from "../src/lib/terminal/links";

const texts = (line: string) => findPathLinks(line).map((match) => match.text);

describe("findPathLinks", () => {
  it("finds absolute, home and dotted paths", () => {
    expect(texts("open /usr/local/bin and ~/notes.txt then ./run.sh ../up/file")).toEqual([
      "/usr/local/bin",
      "~/notes.txt",
      "./run.sh",
      "../up/file",
    ]);
    expect(texts("C:\\Users\\me\\a.txt is here")).toEqual(["C:\\Users\\me\\a.txt"]);
  });

  it("finds relative paths and compiler positions", () => {
    expect(texts("error in src/lib/layout.ts:42:7: bad")).toEqual(["src/lib/layout.ts:42:7"]);
    expect(texts("  --> main.rs:12:3")).toEqual(["main.rs:12:3"]);
    expect(texts("see docs/roadmap.md.")).toEqual(["docs/roadmap.md"]);
    expect(texts("a/b/c")).toEqual(["a/b/c"]);
  });

  it("reports where the match is, without trailing punctuation", () => {
    const [match] = findPathLinks("cat (/etc/hosts), ok");
    expect(match).toEqual({ text: "/etc/hosts", start: 5, end: 15 });
    expect(texts('"/tmp/a b"')).toEqual(["/tmp/a"]);
    expect(texts("path=/var/log/")).toEqual(["/var/log/"]);
  });

  it("leaves URLs to the URL matcher", () => {
    expect(texts("https://example.com/a/b/c.html")).toEqual([]);
    expect(texts("go to http://localhost:3000/app now")).toEqual([]);
    expect(texts("mailto:someone@example.com")).toEqual([]);
  });

  it("does not turn ordinary words and numbers into paths", () => {
    expect(texts("and/or TCP/IP 1/2 24/7 yes/no")).toEqual([]);
    expect(texts("version 1.2.3, ratio 3:4, time 10:45:12")).toEqual([]);
    expect(texts("just some words.")).toEqual([]);
  });

  it("finds several in one line", () => {
    expect(texts("diff /a/one.txt /b/two.txt")).toEqual(["/a/one.txt", "/b/two.txt"]);
    expect(texts("cp a.txt,b.txt --out=/tmp/o")).toEqual(["/tmp/o"]);
  });
});

describe("cells and columns", () => {
  it("maps each character to the column it sits in, wide characters included", () => {
    const cells = [
      { chars: "a", width: 1 },
      { chars: "字", width: 2 },
      { chars: "", width: 0 },
      { chars: "b", width: 1 },
      { chars: "", width: 1 },
    ];
    expect(cellsToLine(cells)).toEqual({ text: "a字b ", columns: [0, 1, 3, 4] });
  });

  it("gives the 1-based inclusive columns of a match after a wide character", () => {
    const cells = [
      { chars: "字", width: 2 },
      { chars: "", width: 0 },
      { chars: " ", width: 1 },
      ...Array.from("/tmp/x", (chars) => ({ chars, width: 1 })),
    ];
    const { text, columns } = cellsToLine(cells);
    const [match] = findPathLinks(text);
    expect(match.text).toBe("/tmp/x");
    // The wide character takes columns 0 and 1, the space is column 2, the path starts at column 3:
    // 1-based, that is 4 through 9.
    expect(columnsOf(match, columns)).toEqual({ start: 4, end: 9 });
  });

  it("copes with a surrogate pair", () => {
    const { text, columns } = cellsToLine([{ chars: "😀", width: 2 }, { chars: "", width: 0 }, { chars: "x", width: 1 }]);
    expect(text).toBe("😀x");
    expect(columns).toEqual([0, 0, 2]);
  });
});

describe("isOpenGesture", () => {
  it("needs ⌘ on macOS and Ctrl elsewhere", () => {
    expect(isOpenGesture({ metaKey: true, ctrlKey: false }, "macos")).toBe(true);
    expect(isOpenGesture({ metaKey: false, ctrlKey: true }, "macos")).toBe(false);
    expect(isOpenGesture({ metaKey: false, ctrlKey: true }, "linux")).toBe(true);
    expect(isOpenGesture({ metaKey: true, ctrlKey: false }, "windows")).toBe(false);
    expect(isOpenGesture({ metaKey: false, ctrlKey: false }, "macos")).toBe(false);
  });
});
