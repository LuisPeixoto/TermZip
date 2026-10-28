/** Rendering only what is on screen: a pure model of a list whose items have different heights. */

export type Key = string | number;

/** What an item that has not been measured yet is assumed to be. */
export const ESTIMATED_HEIGHT = 150;

/** Where each item starts, plus the total at the end, so there is one more offset than items. */
export function offsetsFor(
  keys: readonly Key[],
  heights: ReadonlyMap<Key, number>,
  estimate: number = ESTIMATED_HEIGHT,
): number[] {
  const offsets = new Array<number>(keys.length + 1);
  offsets[0] = 0;
  for (let index = 0; index < keys.length; index += 1) {
    offsets[index + 1] = offsets[index] + (heights.get(keys[index]) ?? estimate);
  }
  return offsets;
}

export interface Range {
  /** First item to render. */
  start: number;
  /** One past the last item to render. */
  end: number;
}

/** The last index whose offset is at or before `position`, by binary search. */
function indexAt(offsets: readonly number[], position: number): number {
  let low = 0;
  let high = offsets.length - 2; // items, not the closing total
  while (low < high) {
    const middle = (low + high + 1) >> 1;
    if (offsets[middle] <= position) low = middle;
    else high = middle - 1;
  }
  return low;
}

/** The items that touch the viewport, plus a few more each side so a fast scroll never shows a gap. */
export function visibleRange(
  offsets: readonly number[],
  scrollTop: number,
  viewport: number,
  overscan = 3,
): Range {
  const count = offsets.length - 1;
  if (count <= 0) return { start: 0, end: 0 };
  const top = Math.max(0, scrollTop);
  const first = indexAt(offsets, top);
  // An item that starts below the viewport's bottom edge is out; one that starts above it is in.
  const last = indexAt(offsets, Math.max(top, top + viewport - 1));
  return { start: Math.max(0, first - overscan), end: Math.min(count, last + 1 + overscan) };
}

/** The item at the top of the viewport and how far into it the viewport starts. */
export interface Anchor {
  key: Key;
  inside: number;
}

export function anchorAt(keys: readonly Key[], offsets: readonly number[], scrollTop: number): Anchor | null {
  if (keys.length === 0) return null;
  const index = indexAt(offsets, Math.max(0, scrollTop));
  return { key: keys[index], inside: Math.max(0, scrollTop) - offsets[index] };
}

/**
 * Where to scroll so that the anchored item stays where it was on screen after items were added above
 * it or measured. `null` when the item is gone.
 */
export function scrollTopFor(keys: readonly Key[], offsets: readonly number[], anchor: Anchor): number | null {
  const index = keys.indexOf(anchor.key);
  return index < 0 ? null : offsets[index] + anchor.inside;
}
