import { describe, expect, it } from "vitest";
import { basename } from "../src/lib/paths";

describe("basename", () => {
  it("handles both kinds of separator and trailing ones", () => {
    expect(basename("/Users/me/project")).toBe("project");
    expect(basename("/Users/me/project/")).toBe("project");
    expect(basename("C:\\Users\\me\\project")).toBe("project");
    expect(basename("C:\\Users\\me\\project\\")).toBe("project");
  });

  it("returns the root itself and nothing for nothing", () => {
    expect(basename("/")).toBe("/");
    expect(basename("")).toBeNull();
    expect(basename(null)).toBeNull();
    expect(basename(undefined)).toBeNull();
  });
});
