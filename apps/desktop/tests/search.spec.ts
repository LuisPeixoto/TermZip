import { describe, expect, it } from "vitest";
import { NO_RESULTS, describeResults, matchColors, regexProblem } from "../src/lib/terminal/search";

describe("describeResults", () => {
  it("says nothing before anything is typed", () => {
    expect(describeResults(NO_RESULTS, false)).toBe("");
  });

  it("counts from one, like a person would", () => {
    expect(describeResults({ index: 0, count: 12 }, true)).toBe("1 of 12");
    expect(describeResults({ index: 11, count: 12 }, true)).toBe("12 of 12");
  });

  it("reports no matches and the highlight limit", () => {
    expect(describeResults({ index: -1, count: 0 }, true)).toBe("No matches");
    expect(describeResults({ index: -1, count: 1000 }, true)).toBe("1000+ matches");
  });
});

describe("regexProblem", () => {
  it("accepts good expressions and the empty one", () => {
    expect(regexProblem("")).toBeNull();
    expect(regexProblem("err(or)?\\s+\\d+")).toBeNull();
  });

  it("explains a bad one without the engine's prefix", () => {
    const problem = regexProblem("(unclosed");
    expect(problem).not.toBeNull();
    expect(problem).not.toMatch(/^Invalid regular expression/);
  });
});

describe("matchColors", () => {
  it("uses the #RRGGBB form the addon requires, in both themes", () => {
    for (const dark of [true, false]) {
      for (const color of Object.values(matchColors(dark))) {
        expect(color).toMatch(/^#[0-9a-f]{6}$/);
      }
    }
    expect(matchColors(true).matchBackground).not.toBe(matchColors(false).matchBackground);
  });
});
