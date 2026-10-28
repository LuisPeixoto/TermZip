import { describe, expect, it } from "vitest";
import type { Profile } from "../src/lib/api";
import { describeProfile, formatEnv, parseEnv, slug, uniqueProfileId } from "../src/lib/profiles";

/** The message parseEnv gives for text it refuses. */
function errorOf(text: string): string {
  const result = parseEnv(text);
  if (!("error" in result)) throw new Error(`expected an error for ${JSON.stringify(text)}`);
  return result.error;
}

describe("slug and ids", () => {
  it("makes lowercase dashed ids", () => {
    expect(slug("Work Laptop!")).toBe("work-laptop");
    expect(slug("  --A__B--  ")).toBe("a-b");
    expect(slug("!!!")).toBe("");
  });

  it("keeps ids unique and never hands out the built-in one", () => {
    expect(uniqueProfileId("Work", [])).toBe("work");
    expect(uniqueProfileId("Work", ["work"])).toBe("work-2");
    expect(uniqueProfileId("work", ["work", "work-2"])).toBe("work-3");
    expect(uniqueProfileId("Default", [])).toBe("profile");
    expect(uniqueProfileId("???", ["profile"])).toBe("profile-2");
  });

  it("agrees with the backend for the names a user is likely to type", () => {
    // The backend derives the same ids (profile.rs: ids_come_from_the_name_and_stay_unique).
    expect(slug("Work Laptop!")).toBe("work-laptop");
    expect(slug("work laptop")).toBe("work-laptop");
  });
});

describe("environment text", () => {
  it("reads KEY=value lines, skipping blanks and comments", () => {
    const result = parseEnv("A=1\n\n# a comment\nB = two words \r\nC=x=y\n");
    expect(result).toEqual({ env: { A: "1", B: " two words ", C: "x=y" } });
  });

  it("explains the first problem and the line it is on", () => {
    expect(errorOf("A=1\nnoequals")).toBe("Line 2 needs the form KEY=value.");
    expect(errorOf("9X=1")).toContain("not a valid variable name");
    expect(errorOf("TERM=dumb")).toContain("managed by TermZip");
    expect(errorOf("termzip_home=/x")).toContain("managed by TermZip");
    expect(errorOf(`A=${"v".repeat(5000)}`)).toContain("too long");
  });

  it("bounds the number of variables", () => {
    const many = Array.from({ length: 70 }, (_, n) => `V${n}=1`).join("\n");
    expect(errorOf(many)).toContain("At most 64");
  });

  it("round-trips through the text form", () => {
    const env = { A: "1", B: "two words" };
    expect(parseEnv(formatEnv(env))).toEqual({ env });
    expect(formatEnv({})).toBe("");
  });
});

describe("describeProfile", () => {
  const base: Profile = { id: "p", name: "P", shell: null, workingDirectory: null, env: {} };

  it("says what differs from the built-in profile", () => {
    expect(describeProfile(base)).toBe("Same as the built-in profile");
    expect(describeProfile({ ...base, shell: "/bin/bash", workingDirectory: "/srv", env: { A: "1" } })).toBe(
      "/bin/bash · /srv · 1 variable",
    );
    expect(describeProfile({ ...base, env: { A: "1", B: "2" } })).toBe("2 variables");
  });
});
