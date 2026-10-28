import type { PaneNode, SplitDirection } from "./api";

/** The shape of the panes in a tab. Data only: ids point at panes, nothing is stored in the tree. */
export type Layout =
  | { kind: "leaf"; id: string }
  | { kind: "split"; id: string; direction: SplitDirection; ratio: number; first: Layout; second: Layout };

/** Mirrors the limits the backend enforces on a saved workspace (`workspace.rs`). */
export const MAX_PANES = 16;
export const MAX_DEPTH = 8;
export const MIN_RATIO = 0.1;
export const MAX_RATIO = 0.9;

export type Side = "left" | "right" | "up" | "down";

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export function leaf(id: string): Layout {
  return { kind: "leaf", id };
}

/** Pane ids from left to right and top to bottom. */
export function leaves(layout: Layout): string[] {
  return layout.kind === "leaf" ? [layout.id] : [...leaves(layout.first), ...leaves(layout.second)];
}

export function leafCount(layout: Layout): number {
  return layout.kind === "leaf" ? 1 : leafCount(layout.first) + leafCount(layout.second);
}

export function depth(layout: Layout): number {
  return layout.kind === "leaf" ? 1 : 1 + Math.max(depth(layout.first), depth(layout.second));
}

export function contains(layout: Layout, id: string): boolean {
  return layout.kind === "leaf"
    ? layout.id === id
    : contains(layout.first, id) || contains(layout.second, id);
}

export function clampRatio(ratio: number): number {
  return Number.isFinite(ratio) ? Math.min(MAX_RATIO, Math.max(MIN_RATIO, ratio)) : 0.5;
}

/** Whether `id` can be split without passing the limits. */
export function canSplit(layout: Layout, id: string): boolean {
  if (!contains(layout, id) || leafCount(layout) >= MAX_PANES) return false;
  return depth(split(layout, id, "row", "probe", "probe-split")) <= MAX_DEPTH;
}

/** Puts a new pane beside `id`: after it, or before it when `before` is set. */
export function split(
  layout: Layout,
  id: string,
  direction: SplitDirection,
  newId: string,
  splitId: string,
  before = false,
): Layout {
  if (layout.kind === "leaf") {
    if (layout.id !== id) return layout;
    const added = leaf(newId);
    return {
      kind: "split",
      id: splitId,
      direction,
      ratio: 0.5,
      first: before ? added : layout,
      second: before ? layout : added,
    };
  }
  const first = split(layout.first, id, direction, newId, splitId, before);
  const second = first === layout.first ? split(layout.second, id, direction, newId, splitId, before) : layout.second;
  return first === layout.first && second === layout.second ? layout : { ...layout, first, second };
}

/** Removes a pane; its sibling takes the space. `null` when it was the only pane. */
export function close(layout: Layout, id: string): Layout | null {
  if (layout.kind === "leaf") return layout.id === id ? null : layout;
  const first = close(layout.first, id);
  const second = close(layout.second, id);
  if (first === null) return second;
  if (second === null) return first;
  return first === layout.first && second === layout.second ? layout : { ...layout, first, second };
}

export function setRatio(layout: Layout, splitId: string, ratio: number): Layout {
  if (layout.kind === "leaf") return layout;
  if (layout.id === splitId) return { ...layout, ratio: clampRatio(ratio) };
  const first = setRatio(layout.first, splitId, ratio);
  const second = setRatio(layout.second, splitId, ratio);
  return first === layout.first && second === layout.second ? layout : { ...layout, first, second };
}

/** Where every pane sits in a unit square, so neighbors can be found by geometry. */
export function rects(layout: Layout, area: Rect = { x: 0, y: 0, w: 1, h: 1 }): Map<string, Rect> {
  if (layout.kind === "leaf") return new Map([[layout.id, area]]);
  const { ratio, direction } = layout;
  const [a, b]: [Rect, Rect] =
    direction === "row"
      ? [
          { ...area, w: area.w * ratio },
          { ...area, x: area.x + area.w * ratio, w: area.w * (1 - ratio) },
        ]
      : [
          { ...area, h: area.h * ratio },
          { ...area, y: area.y + area.h * ratio, h: area.h * (1 - ratio) },
        ];
  return new Map([...rects(layout.first, a), ...rects(layout.second, b)]);
}

const EPSILON = 1e-6;

function overlap(aStart: number, aEnd: number, bStart: number, bEnd: number): number {
  return Math.max(0, Math.min(aEnd, bEnd) - Math.max(aStart, bStart));
}

/** The pane next to `id` in the given direction, or `null` at an edge. */
export function neighbor(layout: Layout, id: string, side: Side): string | null {
  const all = rects(layout);
  const from = all.get(id);
  if (!from) return null;

  let best: { id: string; gap: number; shared: number } | null = null;
  for (const [candidate, rect] of all) {
    if (candidate === id) continue;
    let gap: number;
    let shared: number;
    if (side === "right" || side === "left") {
      gap = side === "right" ? rect.x - (from.x + from.w) : from.x - (rect.x + rect.w);
      shared = overlap(from.y, from.y + from.h, rect.y, rect.y + rect.h);
    } else {
      gap = side === "down" ? rect.y - (from.y + from.h) : from.y - (rect.y + rect.h);
      shared = overlap(from.x, from.x + from.w, rect.x, rect.x + rect.w);
    }
    if (gap < -EPSILON || shared <= EPSILON) continue;
    if (!best || gap < best.gap - EPSILON || (Math.abs(gap - best.gap) <= EPSILON && shared > best.shared)) {
      best = { id: candidate, gap, shared };
    }
  }
  return best?.id ?? null;
}

/** What a pane needs to come back: the profile it started with and where its shell was. */
export interface LeafInfo {
  profile: string | null;
  cwd: string | null;
}

/** The saved form of a layout; `describe` supplies each pane's profile and folder. */
export function toNode(layout: Layout, describe: (id: string) => LeafInfo): PaneNode {
  if (layout.kind === "leaf") return { kind: "leaf", ...describe(layout.id) };
  return {
    kind: "split",
    direction: layout.direction,
    ratio: layout.ratio,
    first: toNode(layout.first, describe),
    second: toNode(layout.second, describe),
  };
}

/** A saved layout back as a tree with fresh ids, and what each new pane should start with. */
export function fromNode(
  node: PaneNode,
  newId: () => string,
): { layout: Layout; panes: Array<{ id: string } & LeafInfo> } {
  if (node.kind === "leaf") {
    const id = newId();
    return { layout: leaf(id), panes: [{ id, profile: node.profile, cwd: node.cwd }] };
  }
  const first = fromNode(node.first, newId);
  const second = fromNode(node.second, newId);
  return {
    layout: {
      kind: "split",
      id: newId(),
      direction: node.direction,
      ratio: clampRatio(node.ratio),
      first: first.layout,
      second: second.layout,
    },
    panes: [...first.panes, ...second.panes],
  };
}
