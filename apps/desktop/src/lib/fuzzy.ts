/**
 * Fuzzy matching for the command palette: the query's letters must appear in the text in order, and
 * the best way of spreading them wins. Letters that start a word or follow each other score higher,
 * so "ntab" finds "New tab" before "Go to the next tab".
 */

export interface FuzzyMatch {
  score: number;
  /** Half-open `[start, end)` stretches of the text that matched, for highlighting. */
  ranges: [number, number][];
}

const WORD_START = 12;
const CONSECUTIVE = 16;
const MATCH = 1;
const GAP = 0.15;
const LEADING = 0.05;

const isAlnum = (char: string) => /[\p{L}\p{N}]/u.test(char);

/** A position where a word begins: the start, after a separator, or a capital after a lowercase letter. */
function startsWord(text: string, at: number): boolean {
  if (at === 0) return true;
  const before = text[at - 1];
  if (!isAlnum(before)) return true;
  return before === before.toLowerCase() && before !== before.toUpperCase() && text[at] !== text[at].toLowerCase();
}

export function fuzzyMatch(query: string, text: string): FuzzyMatch | null {
  const wanted = [...query.toLowerCase().replace(/\s+/g, "")];
  if (wanted.length === 0) return { score: 0, ranges: [] };
  const lower = text.toLowerCase();
  if (wanted.length > lower.length) return null;

  const n = lower.length;
  const m = wanted.length;
  // best[i][j]: the best score with wanted[i] matched at text[j]; from[i][j]: where wanted[i-1] was matched.
  const best: number[][] = Array.from({ length: m }, () => new Array<number>(n).fill(-Infinity));
  const from: number[][] = Array.from({ length: m }, () => new Array<number>(n).fill(-1));

  for (let j = 0; j < n; j += 1) {
    if (lower[j] !== wanted[0]) continue;
    best[0][j] = MATCH + (startsWord(text, j) ? WORD_START : 0) - LEADING * j;
  }
  for (let i = 1; i < m; i += 1) {
    for (let j = i; j < n; j += 1) {
      if (lower[j] !== wanted[i]) continue;
      for (let k = i - 1; k < j; k += 1) {
        if (best[i - 1][k] === -Infinity) continue;
        const consecutive = k === j - 1;
        const score =
          best[i - 1][k] +
          MATCH +
          (consecutive ? CONSECUTIVE : -GAP * Math.min(j - k - 1, 10)) +
          (startsWord(text, j) ? WORD_START : 0);
        if (score > best[i][j]) {
          best[i][j] = score;
          from[i][j] = k;
        }
      }
    }
  }

  let end = -1;
  let score = -Infinity;
  for (let j = 0; j < n; j += 1) {
    if (best[m - 1][j] > score) {
      score = best[m - 1][j];
      end = j;
    }
  }
  if (end < 0) return null;

  const positions: number[] = [];
  for (let i = m - 1, j = end; i >= 0; j = from[i][j], i -= 1) positions.unshift(j);
  const ranges: [number, number][] = [];
  for (const at of positions) {
    const last = ranges[ranges.length - 1];
    if (last && last[1] === at) last[1] = at + 1;
    else ranges.push([at, at + 1]);
  }
  return { score, ranges };
}

export interface Ranked<T> {
  item: T;
  ranges: [number, number][];
}

/**
 * The items that match, best first. Equal scores keep the order they came in. With no query every item is
 * listed: those in `recent` first (most recent first), then the rest as given.
 */
export function rank<T>(
  items: readonly T[],
  query: string,
  title: (item: T) => string,
  id: (item: T) => string,
  recent: readonly string[] = [],
): Ranked<T>[] {
  if (!query.trim()) {
    const order = (item: T) => {
      const at = recent.indexOf(id(item));
      return at < 0 ? Infinity : at;
    };
    return items
      .map((item, index) => ({ item, index }))
      .sort((a, b) => order(a.item) - order(b.item) || a.index - b.index)
      .map(({ item }) => ({ item, ranges: [] }));
  }
  const found: { item: T; ranges: [number, number][]; score: number; index: number }[] = [];
  items.forEach((item, index) => {
    const match = fuzzyMatch(query, title(item));
    if (match) found.push({ item, ranges: match.ranges, score: match.score, index });
  });
  return found.sort((a, b) => b.score - a.score || a.index - b.index).map(({ item, ranges }) => ({ item, ranges }));
}

export interface Segment {
  text: string;
  hit: boolean;
}

/** The text cut at the matched stretches, so each can be drawn on its own. */
export function highlight(text: string, ranges: readonly [number, number][]): Segment[] {
  const segments: Segment[] = [];
  let at = 0;
  for (const [start, end] of ranges) {
    if (start > at) segments.push({ text: text.slice(at, start), hit: false });
    segments.push({ text: text.slice(start, end), hit: true });
    at = end;
  }
  if (at < text.length) segments.push({ text: text.slice(at), hit: false });
  return segments;
}
