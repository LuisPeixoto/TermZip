import { describe, expect, it } from "vitest";
import type { Block } from "../src/lib/api";
import { appendBlock, blockStatus, filterBlocks, MAX_BLOCKS } from "../src/lib/blocks";

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

function made(index: number, command: string, output: string | null, exitCode: number | null): Block {
  return { ...block(index, command), output, exitCode };
}

describe("blockStatus", () => {
  it("tells success from failure from not-known", () => {
    expect(blockStatus(made(0, "a", null, 0))).toBe("ok");
    expect(blockStatus(made(0, "a", null, 2))).toBe("failed");
    expect(blockStatus(made(0, "a", null, null))).toBe("unknown");
  });
});

describe("filterBlocks", () => {
  const blocks = [
    made(0, "git status", "On branch main\nnothing to commit", 0),
    made(1, "npm test", "\x1b[31mFAIL\x1b[0m src/layout.spec.ts\n1 failed", 1),
    made(2, "ls missing", "ls: missing: No such file or directory", 1),
    made(3, "sleep 30", null, null),
  ];
  const indexes = (list: Block[]) => list.map((entry) => entry.index);

  it("returns everything when nothing is asked", () => {
    expect(indexes(filterBlocks(blocks, "", "all"))).toEqual([0, 1, 2, 3]);
    expect(indexes(filterBlocks(blocks, "   ", "all"))).toEqual([0, 1, 2, 3]);
  });

  it("finds text in the command or in the output, ignoring case", () => {
    expect(indexes(filterBlocks(blocks, "STATUS", "all"))).toEqual([0]);
    expect(indexes(filterBlocks(blocks, "no such file", "all"))).toEqual([2]);
    expect(indexes(filterBlocks(blocks, "layout", "all"))).toEqual([1]);
  });

  it("searches what is seen, not the color codes", () => {
    // "31m" is only in the escape sequence, "FAIL" is on screen.
    expect(indexes(filterBlocks(blocks, "31m", "all"))).toEqual([]);
    expect(indexes(filterBlocks(blocks, "fail src", "all"))).toEqual([1]);
    expect(indexes(filterBlocks(blocks, "FAIL", "all"))).toEqual([1]);
  });

  it("filters by status, and combines it with the text", () => {
    expect(indexes(filterBlocks(blocks, "", "ok"))).toEqual([0]);
    expect(indexes(filterBlocks(blocks, "", "failed"))).toEqual([1, 2]);
    expect(indexes(filterBlocks(blocks, "missing", "failed"))).toEqual([2]);
    expect(indexes(filterBlocks(blocks, "missing", "ok"))).toEqual([]);
  });

  it("keeps code-less blocks out of ok and failed", () => {
    expect(indexes(filterBlocks(blocks, "sleep", "all"))).toEqual([3]);
    expect(indexes(filterBlocks(blocks, "sleep", "ok"))).toEqual([]);
    expect(indexes(filterBlocks(blocks, "sleep", "failed"))).toEqual([]);
  });

  it("does not change the list it was given", () => {
    const before = [...blocks];
    filterBlocks(blocks, "git", "ok");
    expect(blocks).toEqual(before);
  });
});
