import { describe, expect, it } from "vitest";
import { anchorAt, offsetsFor, scrollTopFor, visibleRange } from "../src/lib/virtual";

describe("offsetsFor", () => {
  it("adds up measured heights and estimates the rest", () => {
    const heights = new Map<number, number>([
      [10, 100],
      [11, 50],
    ]);
    expect(offsetsFor([10, 11, 12], heights, 30)).toEqual([0, 100, 150, 180]);
    expect(offsetsFor([], new Map(), 30)).toEqual([0]);
  });
});

describe("visibleRange", () => {
  // Ten items of 100px each: item i covers [100 i, 100 i + 100).
  const offsets = offsetsFor(Array.from({ length: 10 }, (_, n) => n), new Map(), 100);

  it("covers the viewport and nothing more, without overscan", () => {
    expect(visibleRange(offsets, 0, 250, 0)).toEqual({ start: 0, end: 3 });
    expect(visibleRange(offsets, 150, 100, 0)).toEqual({ start: 1, end: 3 });
    expect(visibleRange(offsets, 200, 100, 0)).toEqual({ start: 2, end: 3 });
  });

  it("adds overscan each side and stays inside the list", () => {
    expect(visibleRange(offsets, 500, 100, 2)).toEqual({ start: 3, end: 8 });
    expect(visibleRange(offsets, 0, 100, 3)).toEqual({ start: 0, end: 4 });
    expect(visibleRange(offsets, 900, 100, 3)).toEqual({ start: 6, end: 10 });
  });

  it("copes with a position past the end, a negative one, and an empty list", () => {
    expect(visibleRange(offsets, 99_999, 100, 0)).toEqual({ start: 9, end: 10 });
    expect(visibleRange(offsets, -50, 100, 0)).toEqual({ start: 0, end: 1 });
    expect(visibleRange([0], 0, 500, 3)).toEqual({ start: 0, end: 0 });
  });

  it("renders a handful of items out of thousands", () => {
    const many = offsetsFor(Array.from({ length: 5000 }, (_, n) => n), new Map(), 120);
    const range = visibleRange(many, 120 * 2500, 800, 3);
    expect(range.end - range.start).toBeLessThan(20);
    expect(range.start).toBeGreaterThan(2400);
  });
});

describe("anchoring the scroll position", () => {
  it("keeps the same item in place when items are added above it", () => {
    const keys = [3, 2, 1];
    const offsets = offsetsFor(keys, new Map(), 100);
    // The viewport starts 40px into item 2.
    const anchor = anchorAt(keys, offsets, 140)!;
    expect(anchor).toEqual({ key: 2, inside: 40 });

    // A new block appears on top and is taller than the estimate once measured.
    const after = [4, 3, 2, 1];
    const heights = new Map<number, number>([[4, 220]]);
    const next = offsetsFor(after, heights, 100);
    expect(scrollTopFor(after, next, anchor)).toBe(220 + 100 + 40);
  });

  it("says nothing when the anchored item is gone, or the list is empty", () => {
    const anchor = { key: 7, inside: 10 };
    expect(scrollTopFor([1, 2], offsetsFor([1, 2], new Map(), 100), anchor)).toBeNull();
    expect(anchorAt([], [0], 50)).toBeNull();
  });
});
