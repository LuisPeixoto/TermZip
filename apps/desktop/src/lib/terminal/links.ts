/** Finding clickable text in a terminal line. Pure: the terminal and the backend do the rest. */

export interface PathMatch {
  /** The text to hand to the backend, including a `:line:column` suffix if there was one. */
  text: string;
  /** Where it starts in the line, and one past where it ends. */
  start: number;
  end: number;
}

// `=` and `,` are left out: they separate a path from what is around it (`--config=/etc/x`, `a.txt,b.txt`).
const SEGMENT = String.raw`[\w.@%+~-]+`;
const POSITION = String.raw`(?::\d+(?::\d+)?)?`;
// 1. a path that starts somewhere recognisable: /abs, ~/home, ./here, ../up, C:\drive
// 2. a relative path with at least one separator: src/lib/a.ts
// 3. a bare file name with a position, as compilers print it: main.rs:12:3
const PATTERN = new RegExp(
  String.raw`(?:~|\.{1,2}|[A-Za-z]:)?[\\/]${SEGMENT}(?:[\\/]${SEGMENT})*[\\/]?${POSITION}` +
    String.raw`|${SEGMENT}(?:[\\/]${SEGMENT})+[\\/]?${POSITION}` +
    String.raw`|${SEGMENT}\.[A-Za-z]\w{0,7}:\d+(?::\d+)?`,
  "g",
);

// Text that may sit right before a path. Anything else (a letter, a slash, a colon) means the
// "path" is the tail of something else: `https://host/a/b` must not yield `//host/a/b`.
const BOUNDARY = /[\s'"`()[\]{}<>=,;|]/;
const TRAILING = /[.,;:!?)\]}>'"]+$/;

function plausible(text: string): boolean {
  if (/^(?:~|\.{1,2}|[A-Za-z]:)?[\\/]/.test(text)) return true;
  // Without a recognisable start, `and/or` and `1/2` look like paths. Ask for depth or an extension.
  const body = text.replace(/:\d+(?::\d+)?$/, "");
  const separators = (body.match(/[\\/]/g) ?? []).length;
  return separators >= 2 || /\.[A-Za-z0-9]+[\\/]?$/.test(body) || /:\d+/.test(text);
}

export function findPathLinks(line: string): PathMatch[] {
  const found: PathMatch[] = [];
  for (const match of line.matchAll(PATTERN)) {
    const index = match.index ?? 0;
    if (index > 0 && !BOUNDARY.test(line[index - 1])) continue;
    const text = match[0].replace(TRAILING, "");
    if (!text || !plausible(text)) continue;
    found.push({ text, start: index, end: index + text.length });
  }
  return found;
}

/** One cell of the terminal buffer, as the part of the API this needs. */
export interface Cell {
  chars: string;
  /** 2 for the first half of a wide character, 0 for its empty second half. */
  width: number;
}

/**
 * The text of a row and, for each character of it, the 0-based column it sits in. Wide characters
 * take two columns and an empty cell is a space, so a position in the text is not a column.
 */
export function cellsToLine(cells: readonly Cell[]): { text: string; columns: number[] } {
  let text = "";
  const columns: number[] = [];
  cells.forEach((cell, column) => {
    if (cell.width === 0) return;
    const chars = cell.chars === "" ? " " : cell.chars;
    text += chars;
    for (let index = 0; index < chars.length; index += 1) columns.push(column);
  });
  return { text, columns };
}

/** The 1-based, inclusive columns xterm wants for a match found in a row. */
export function columnsOf(match: PathMatch, columns: readonly number[]): { start: number; end: number } {
  return { start: columns[match.start] + 1, end: columns[match.end - 1] + 1 };
}

/** Clicking a link only opens it with ⌘ held (Ctrl elsewhere), so selecting text that contains one still works. */
export function isOpenGesture(event: Pick<MouseEvent, "metaKey" | "ctrlKey">, platform: string): boolean {
  return platform === "macos" ? event.metaKey : event.ctrlKey;
}
