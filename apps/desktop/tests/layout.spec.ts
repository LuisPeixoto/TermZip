import { describe, expect, it } from "vitest";
import type { PaneNode } from "../src/lib/api";
import {
  MAX_DEPTH,
  MAX_PANES,
  canSplit,
  clampRatio,
  close,
  contains,
  dividers,
  depth,
  fromNode,
  leaf,
  leafCount,
  leaves,
  neighbor,
  ratioAt,
  rects,
  setRatio,
  split,
  toNode,
  type Layout,
} from "../src/lib/layout";

let counter = 0;
const fresh = () => `n${(counter += 1)}`;

/** a | (b over c) */
function sample(): Layout {
  return split(split(leaf("a"), "a", "row", "b", "s1"), "b", "column", "c", "s2");
}

describe("building and reading a layout", () => {
  it("lists panes left to right, top to bottom", () => {
    expect(leaves(sample())).toEqual(["a", "b", "c"]);
    expect(leafCount(sample())).toBe(3);
    expect(depth(sample())).toBe(3);
    expect(contains(sample(), "c")).toBe(true);
    expect(contains(sample(), "x")).toBe(false);
  });

  it("splits a pane in place and can put the new one before it", () => {
    const after = split(leaf("a"), "a", "row", "b", "s");
    expect(leaves(after)).toEqual(["a", "b"]);
    const before = split(leaf("a"), "a", "row", "b", "s", true);
    expect(leaves(before)).toEqual(["b", "a"]);
  });

  it("leaves the tree untouched (same object) when the pane is not there", () => {
    const tree = sample();
    expect(split(tree, "nope", "row", "z", "sz")).toBe(tree);
    expect(close(tree, "nope")).toBe(tree);
    expect(setRatio(tree, "nope", 0.3)).toBe(tree);
  });

  it("never mutates what it was given", () => {
    const tree = sample();
    const snapshot = JSON.stringify(tree);
    split(tree, "a", "column", "z", "sz");
    close(tree, "b");
    setRatio(tree, "s1", 0.2);
    expect(JSON.stringify(tree)).toBe(snapshot);
  });
});

describe("closing a pane", () => {
  it("gives its space to the sibling", () => {
    expect(leaves(close(sample(), "b")!)).toEqual(["a", "c"]);
    const afterBoth = close(close(sample(), "b")!, "c")!;
    expect(afterBoth).toEqual(leaf("a"));
  });

  it("returns null when the last pane closes", () => {
    expect(close(leaf("a"), "a")).toBeNull();
  });
});

describe("ratios", () => {
  it("are clamped to a usable range", () => {
    expect(clampRatio(0)).toBe(0.1);
    expect(clampRatio(5)).toBe(0.9);
    expect(clampRatio(Number.NaN)).toBe(0.5);
    const resized = setRatio(sample(), "s1", 0.02);
    expect(resized.kind === "split" && resized.ratio).toBe(0.1);
  });
});

describe("limits", () => {
  it("stops splitting at the pane limit", () => {
    let tree: Layout = leaf("p0");
    let next = 1;
    // A balanced build keeps depth low so only the pane limit applies.
    const ids = ["p0"];
    while (leafCount(tree) < MAX_PANES) {
      const target = ids[(next - 1) % ids.length];
      tree = split(tree, target, next % 2 ? "row" : "column", `p${next}`, `s${next}`);
      ids.push(`p${next}`);
      next += 1;
    }
    expect(leafCount(tree)).toBe(MAX_PANES);
    expect(canSplit(tree, "p0")).toBe(false);
  });

  it("stops splitting at the nesting limit", () => {
    let tree: Layout = leaf("p0");
    for (let level = 1; level < MAX_DEPTH; level += 1) {
      tree = split(tree, `p${level - 1}`, "row", `p${level}`, `s${level}`);
    }
    expect(depth(tree)).toBe(MAX_DEPTH);
    expect(canSplit(tree, `p${MAX_DEPTH - 1}`)).toBe(false);
    expect(canSplit(leaf("a"), "a")).toBe(true);
    expect(canSplit(leaf("a"), "missing")).toBe(false);
  });
});

describe("neighbors", () => {
  // a | (b over c)
  it("finds the pane next to another, by geometry", () => {
    const tree = sample();
    expect(neighbor(tree, "a", "right")).toBe("b");
    expect(neighbor(tree, "b", "left")).toBe("a");
    expect(neighbor(tree, "c", "left")).toBe("a");
    expect(neighbor(tree, "b", "down")).toBe("c");
    expect(neighbor(tree, "c", "up")).toBe("b");
  });

  it("is null at an edge", () => {
    const tree = sample();
    expect(neighbor(tree, "a", "left")).toBeNull();
    expect(neighbor(tree, "a", "up")).toBeNull();
    expect(neighbor(tree, "b", "up")).toBeNull();
    expect(neighbor(tree, "c", "right")).toBeNull();
    expect(neighbor(tree, "missing", "right")).toBeNull();
  });

  it("prefers the pane that shares the most edge", () => {
    // (a over b) | c: c touches both on its left, so the taller one wins.
    const column = (ratio: number): Layout => ({
      kind: "split",
      id: "row",
      direction: "row",
      ratio: 0.5,
      first: { kind: "split", id: "col", direction: "column", ratio, first: leaf("a"), second: leaf("b") },
      second: leaf("c"),
    });
    expect(neighbor(column(0.3), "c", "left")).toBe("b");
    expect(neighbor(column(0.8), "c", "left")).toBe("a");
  });

  it("computes rectangles that tile the square", () => {
    const area = [...rects(sample()).values()].reduce((sum, r) => sum + r.w * r.h, 0);
    expect(area).toBeCloseTo(1, 6);
  });
});

describe("the saved form", () => {
  it("round-trips through a node, with fresh ids", () => {
    const tree = sample();
    const info = (id: string) => ({ profile: id === "b" ? "work" : null, cwd: `/${id}` });
    const node = toNode(tree, info);

    const restored = fromNode(node, fresh);

    expect(restored.panes.map((p) => [p.profile, p.cwd])).toEqual([
      [null, "/a"],
      ["work", "/b"],
      [null, "/c"],
    ]);
    expect(leaves(restored.layout)).toEqual(restored.panes.map((p) => p.id));
    expect(leaves(restored.layout)).not.toContain("a");
    expect(toNode(restored.layout, (id) => {
      const pane = restored.panes.find((p) => p.id === id)!;
      return { profile: pane.profile, cwd: pane.cwd };
    })).toEqual(node);
  });

  it("clamps a hostile ratio when restoring", () => {
    const node: PaneNode = {
      kind: "split",
      direction: "row",
      ratio: 7,
      first: { kind: "leaf", profile: null, cwd: null },
      second: { kind: "leaf", profile: null, cwd: null },
    };
    const { layout } = fromNode(node, fresh);
    expect(layout.kind === "split" && layout.ratio).toBe(0.9);
  });
});

describe("dividers", () => {
  it("has one per split, with the area that split covers", () => {
    // a | (b over c): the outer split covers everything, the inner one the right half.
    const lines = dividers(sample());
    expect(lines.map((line) => [line.id, line.direction])).toEqual([
      ["s1", "row"],
      ["s2", "column"],
    ]);
    expect(lines[0].area).toEqual({ x: 0, y: 0, w: 1, h: 1 });
    expect(lines[1].area).toEqual({ x: 0.5, y: 0, w: 0.5, h: 1 });
    expect(dividers(leaf("a"))).toEqual([]);
  });

  it("turns a pointer position into a ratio of its own area, clamped", () => {
    const [outer, inner] = dividers(sample());
    expect(ratioAt(outer, 0.25, 0.9)).toBeCloseTo(0.25);
    // Inside the right half, x = 0.75 is the middle of that half.
    expect(ratioAt(inner, 0.9, 0.75)).toBeCloseTo(0.75);
    expect(ratioAt(outer, -4, 0)).toBe(0.1);
    expect(ratioAt(outer, 9, 0)).toBe(0.9);
  });
});
