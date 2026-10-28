/** ANSI SGR parser producing safe, structured spans (never raw HTML). */

export interface AnsiStyle {
  fg?: string;
  bg?: string;
  bold?: boolean;
  dim?: boolean;
  italic?: boolean;
  underline?: boolean;
  inverse?: boolean;
}

export interface AnsiSpan {
  text: string;
  style: AnsiStyle;
}

export interface AnsiPalette {
  foreground: string;
  background: string;
  colors: string[];
}

export const DEFAULT_PALETTE: AnsiPalette = {
  foreground: "#fafafa",
  background: "#09090b",
  colors: [
    "#3f3f46",
    "#f87171",
    "#34d399",
    "#fbbf24",
    "#60a5fa",
    "#c084fc",
    "#22d3ee",
    "#e4e4e7",
    "#71717a",
    "#fca5a5",
    "#6ee7b7",
    "#fde68a",
    "#93c5fd",
    "#d8b4fe",
    "#67e8f9",
    "#fafafa",
  ],
};

interface State extends AnsiStyle {}

function resetState(): State {
  return {};
}

function colorFromIndex(index: number, palette: AnsiPalette): string | undefined {
  if (index >= 0 && index < 16) return palette.colors[index];
  if (index >= 16 && index <= 231) {
    const value = index - 16;
    const component = (part: number) => (part === 0 ? 0 : 55 + part * 40);
    return `rgb(${component(Math.floor(value / 36))}, ${component(Math.floor((value % 36) / 6))}, ${component(value % 6)})`;
  }
  if (index >= 232 && index <= 255) {
    const gray = 8 + (index - 232) * 10;
    return `rgb(${gray}, ${gray}, ${gray})`;
  }
  return undefined;
}

function applySgr(state: State, params: number[], palette: AnsiPalette): State {
  let next: State = { ...state };
  for (let i = 0; i < params.length; i += 1) {
    const code = params[i];
    switch (code) {
      case 0:
        // A reset only clears what came before it: `0;31` is "reset, then red".
        next = resetState();
        break;
      case 1:
        next.bold = true;
        break;
      case 2:
        next.dim = true;
        break;
      case 3:
        next.italic = true;
        break;
      case 4:
        next.underline = true;
        break;
      case 7:
        next.inverse = true;
        break;
      case 22:
        next.bold = false;
        next.dim = false;
        break;
      case 23:
        next.italic = false;
        break;
      case 24:
        next.underline = false;
        break;
      case 27:
        next.inverse = false;
        break;
      case 30:
      case 31:
      case 32:
      case 33:
      case 34:
      case 35:
      case 36:
      case 37:
        next.fg = palette.colors[code - 30];
        break;
      case 39:
        next.fg = undefined;
        break;
      case 40:
      case 41:
      case 42:
      case 43:
      case 44:
      case 45:
      case 46:
      case 47:
        next.bg = palette.colors[code - 40];
        break;
      case 49:
        next.bg = undefined;
        break;
      case 90:
      case 91:
      case 92:
      case 93:
      case 94:
      case 95:
      case 96:
      case 97:
        next.fg = palette.colors[code - 90 + 8];
        break;
      case 100:
      case 101:
      case 102:
      case 103:
      case 104:
      case 105:
      case 106:
      case 107:
        next.bg = palette.colors[code - 100 + 8];
        break;
      case 38:
      case 48: {
        const target = code === 38 ? "fg" : "bg";
        if (params[i + 1] === 5 && params[i + 2] != null) {
          next[target] = colorFromIndex(params[i + 2], palette);
          i += 2;
        } else if (params[i + 1] === 2 && params[i + 4] != null) {
          next[target] = `rgb(${params[i + 2]}, ${params[i + 3]}, ${params[i + 4]})`;
          i += 4;
        }
        break;
      }
      default:
        break;
    }
  }
  return next;
}

function pushSpan(spans: AnsiSpan[], text: string, style: State): void {
  if (!text) return;
  const last = spans[spans.length - 1];
  if (last && sameStyle(last.style, style)) {
    last.text += text;
  } else {
    spans.push({ text, style: { ...style } });
  }
}

function sameStyle(a: AnsiStyle, b: AnsiStyle): boolean {
  return (
    a.fg === b.fg &&
    a.bg === b.bg &&
    !!a.bold === !!b.bold &&
    !!a.dim === !!b.dim &&
    !!a.italic === !!b.italic &&
    !!a.underline === !!b.underline &&
    !!a.inverse === !!b.inverse
  );
}

/**
 * Parse a captured output string into styled spans. Unknown escape sequences
 * are dropped, carriage returns restart the current line, and backspaces erase
 * the previous character — matching what a user expects from a static view.
 */
export function ansiToSpans(input: string, palette: AnsiPalette = DEFAULT_PALETTE): AnsiSpan[] {
  let state = resetState();
  let spans: AnsiSpan[] = [];
  let index = 0;
  let column = 0;

  const eraseLast = () => {
    const last = spans[spans.length - 1];
    if (!last) return;
    last.text = last.text.slice(0, -1);
    if (!last.text) spans.pop();
    column = Math.max(0, column - 1);
  };

  const newline = () => {
    spans.push({ text: "\n", style: { ...state } });
    column = 0;
  };

  while (index < input.length) {
    const char = input[index];
    if (char === "\x1b") {
      const next = input[index + 1];
      if (next === "[") {
        let end = index + 2;
        while (end < input.length && !/[@-~]/.test(input[end])) end += 1;
        const final = input[end];
        const body = input.slice(index + 2, end);
        if (final === "m") {
          const params = body
            .split(";")
            .map((part) => (part === "" ? 0 : Number.parseInt(part, 10)))
            .filter((value) => Number.isFinite(value));
          state = applySgr(state, params.length ? params : [0], palette);
        }
        index = end + 1;
        continue;
      }
      if (next === "]") {
        let end = index + 2;
        while (end < input.length) {
          if (input[end] === "\x07") break;
          if (input[end] === "\x1b" && input[end + 1] === "\\") {
            end += 1;
            break;
          }
          end += 1;
        }
        index = end + 1;
        continue;
      }
      index += 2;
      continue;
    }

    if (char === "\r") {
      // A bare carriage return rewrites the line; show it as a new line.
      if (input[index + 1] !== "\n") newline();
      index += 1;
      continue;
    }
    if (char === "\b") {
      eraseLast();
      index += 1;
      continue;
    }
    if (char === "\t") {
      const spaces = 8 - (column % 8);
      pushSpan(spans, " ".repeat(spaces), state);
      column += spaces;
      index += 1;
      continue;
    }
    if (char === "\n") {
      newline();
      index += 1;
      continue;
    }
    if (char < " " && char !== "\n") {
      index += 1;
      continue;
    }

    pushSpan(spans, char, state);
    column += 1;
    index += 1;
  }

  return spans;
}
const CSI = /\x1b\[[0-?]*[ -/]*[@-~]/g;
const OSC = /\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)/g;
const OTHER_ESCAPES = /\x1b[@-Z\\-_]/g;

/** The text a person sees: colors, cursor moves and window-title sequences removed. */
export function stripAnsi(input: string): string {
  return input.replace(OSC, "").replace(CSI, "").replace(OTHER_ESCAPES, "");
}
