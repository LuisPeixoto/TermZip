import { describe, expect, it } from "vitest";
import type { Block } from "../src/lib/api";
import { appendBlock, MAX_BLOCKS } from "../src/lib/blocks";

function block(index: number, command = `cmd ${index}`): Block {
  return {
    index,
    command,
    cwd: null,
    exitCode: 0,
    startedAtMs: null,
    durationMs: null,
    outputChars: 0,
    truncated: false,
    output: null,
  };
}

describe("appendBlock", () => {
  it("appends in order and returns a new array", () => {
    const first = [block(0)];
    const next = appendBlock(first, block(1));
    expect(next.map((entry) => entry.index)).toEqual([0, 1]);
    expect(next).not.toBe(first);
    expect(first).toHaveLength(1);
  });

  it("keeps only the newest blocks past the cap", () => {
    let list: Block[] = [];
    for (let index = 0; index < MAX_BLOCKS + 25; index += 1) list = appendBlock(list, block(index));
    expect(list).toHaveLength(MAX_BLOCKS);
    expect(list[0].index).toBe(25);
    expect(list.at(-1)?.index).toBe(MAX_BLOCKS + 24);
  });

  it("replaces a block that arrives twice instead of duplicating it", () => {
    const list = appendBlock([block(0), block(1)], block(1, "updated"));
    expect(list.map((entry) => entry.index)).toEqual([0, 1]);
    expect(list.at(-1)?.command).toBe("updated");
  });
});
