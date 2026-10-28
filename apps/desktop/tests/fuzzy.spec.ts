import { describe, expect, it } from "vitest";
import { fuzzyMatch, rank } from "../src/lib/fuzzy";

const score = (query: string, text: string) => fuzzyMatch(query, text)?.score ?? -Infinity;

describe("fuzzy matching", () => {
  it("needs the letters in order, in any case", () => {
    expect(fuzzyMatch("spr", "Split to the right")).not.toBeNull();
    expect(fuzzyMatch("SPR", "split to the right")).not.toBeNull();
    expect(fuzzyMatch("rps", "Split to the right")).toBeNull();
    expect(fuzzyMatch("xyz", "Split to the right")).toBeNull();
  });

  it("ignores spaces in the query and matches everything for an empty one", () => {
    expect(fuzzyMatch("split right", "Split to the right")).not.toBeNull();
    expect(fuzzyMatch("", "anything")).toEqual({ score: 0, ranges: [] });
    expect(fuzzyMatch("   ", "anything")).toEqual({ score: 0, ranges: [] });
  });

  it("rejects a query longer than the text, and survives odd characters", () => {
    expect(fuzzyMatch("a very long query", "short")).toBeNull();
    expect(fuzzyMatch("(*)", "New tab")).toBeNull();
    expect(fuzzyMatch("é", "Café")).not.toBeNull();
    expect(fuzzyMatch("a", "")).toBeNull();
  });

  it("prefers a prefix, word starts and letters that follow each other", () => {
    expect(score("new", "New tab")).toBeGreaterThan(score("new", "Renew the lease"));
    expect(score("nt", "New tab")).toBeGreaterThan(score("nt", "Restart the shell"));
    expect(score("tab", "New tab")).toBeGreaterThan(score("tab", "Toggle the abacus"));
    expect(score("split", "Split downward")).toBeGreaterThan(score("split", "Show a plain list"));
  });

  it("recognises words in camel case", () => {
    expect(score("ss", "splitStuff")).toBeGreaterThan(score("ss", "passes"));
  });

  it("says which stretches matched, merging letters that touch", () => {
    expect(fuzzyMatch("new", "New tab")?.ranges).toEqual([[0, 3]]);
    expect(fuzzyMatch("nt", "New tab")?.ranges).toEqual([
      [0, 1],
      [4, 5],
    ]);
    expect(fuzzyMatch("tab", "New tab")?.ranges).toEqual([[4, 7]]);
  });
});

describe("ranking entries", () => {
  const items = [
    { id: "a", title: "Split to the right" },
    { id: "b", title: "Split downward" },
    { id: "c", title: "Restart the shell" },
    { id: "d", title: "Find in the terminal" },
  ];
  const by = (query: string, recent: string[] = []) =>
    rank(items, query, (item) => item.title, (item) => item.id, recent).map((entry) => entry.item.id);

  it("lists only what matches, best first", () => {
    expect(by("split")).toEqual(["a", "b"]);
    expect(by("rs")).toContain("c");
    expect(by("zzz")).toEqual([]);
  });

  it("keeps the given order between equal matches", () => {
    expect(by("split r")[0]).toBe("a");
    expect(by("t")).toEqual(by("t"));
  });

  it("puts what was used last first when nothing is typed, and the rest as given", () => {
    expect(by("")).toEqual(["a", "b", "c", "d"]);
    expect(by("", ["c"])).toEqual(["c", "a", "b", "d"]);
    expect(by("", ["d", "c"])).toEqual(["d", "c", "a", "b"]);
    expect(by("   ", ["b"])).toEqual(["b", "a", "c", "d"]);
  });

  it("ignores recents for a query: the match decides", () => {
    expect(by("split", ["b"])).toEqual(["a", "b"]);
  });

  it("returns the ranges for highlighting", () => {
    const [first] = rank(items, "split", (item) => item.title, (item) => item.id);
    expect(first.ranges).toEqual([[0, 5]]);
  });
});
