import { describe, expect, it } from "vitest";
import type { Profile } from "../src/lib/api";
import {
  describeProfile,
  describeRemote,
  formatEnv,
  formatFolders,
  parseEnv,
  parseFolders,
  parseFontSize,
  parsePort,
  slug,
  uniqueProfileId,
} from "../src/lib/profiles";

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

describe("a profile's look", () => {
  const base: Profile = { id: "x", name: "X", shell: null, workingDirectory: null, env: {} };

  it("reads the font size field: empty inherits, otherwise a whole number in range", () => {
    expect(parseFontSize("")).toEqual({ size: null });
    expect(parseFontSize("  ")).toEqual({ size: null });
    expect(parseFontSize("16")).toEqual({ size: 16 });
    expect(parseFontSize(" 10 ")).toEqual({ size: 10 });
    for (const bad of ["9", "29", "12.5", "big", "1e1", "-14"]) {
      const result = parseFontSize(bad);
      expect("error" in result, bad).toBe(true);
    }
  });

  it("lists the look next to where the profile starts", () => {
    const themes = new Map([["dracula", "Dracula"]]);
    const styled: Profile = {
      ...base,
      theme: "dracula",
      fontSize: 16,
      fontFamily: "Fira Code",
      cursorStyle: "bar",
    };

    expect(describeProfile(styled, themes)).toBe("Dracula · 16px · Fira Code · bar cursor");
    expect(describeProfile({ ...styled, shell: "/bin/zsh" }, themes)).toBe(
      "/bin/zsh · Dracula · 16px · Fira Code · bar cursor",
    );
  });

  it("says so when the theme of a profile is gone", () => {
    expect(describeProfile({ ...base, theme: "deleted" })).toBe("deleted (missing)");
  });
});

describe("the folders of a profile", () => {
  it("reads one folder per line, trimmed, without blanks or repeats", () => {
    expect(parseFolders("")).toEqual({ folders: [] });
    expect(parseFolders("  ~/work/api \n\n/srv/app\r\n~/work/api\n")).toEqual({ folders: ["~/work/api", "/srv/app"] });
  });

  it("refuses what the backend would refuse", () => {
    expect("error" in parseFolders("/a\u0007b")).toBe(true);
    expect("error" in parseFolders("x".repeat(1025))).toBe(true);
    const many = Array.from({ length: 17 }, (_, n) => `/f${n}`).join("\n");
    expect(parseFolders(many)).toEqual({ error: "At most 16 folders per profile." });
    expect("folders" in parseFolders(Array.from({ length: 16 }, (_, n) => `/f${n}`).join("\n"))).toBe(true);
  });

  it("writes them back one per line, also for a profile that never had any", () => {
    expect(formatFolders(["~/a", "/b"])).toBe("~/a\n/b");
    expect(formatFolders(undefined)).toBe("");
  });

  it("shows in the list that a profile is tied to folders", () => {
    const base: Profile = { id: "p", name: "P", shell: null, workingDirectory: null, env: {} };
    expect(describeProfile({ ...base, folders: ["~/work/api"] })).toBe("for ~/work/api");
    expect(describeProfile({ ...base, folders: ["/a", "/b"], env: { A: "1" } })).toBe("for 2 folders · 1 variable");
  });
});

describe("remote profiles", () => {
  const base: Profile = { id: "p", name: "P", shell: null, workingDirectory: null, env: {} };

  it("reads the port field: empty is ssh's own, otherwise 1 to 65535", () => {
    expect(parsePort("")).toEqual({ port: null });
    expect(parsePort(" 2222 ")).toEqual({ port: 2222 });
    expect(parsePort("65535")).toEqual({ port: 65535 });
    for (const bad of ["0", "65536", "-1", "22.5", "ssh", "1e3"]) {
      expect("error" in parsePort(bad), bad).toBe(true);
    }
  });

  it("says where an SSH profile connects", () => {
    expect(describeRemote({ ...base, kind: "ssh", host: "build-box" })).toBe("ssh build-box");
    expect(describeRemote({ ...base, kind: "ssh", host: "10.0.0.7", user: "deploy", port: 2222 })).toBe(
      "ssh deploy@10.0.0.7:2222",
    );
    expect(describeRemote({ ...base, kind: "ssh" })).toBe("ssh (no host)");
  });

  it("says which distribution a WSL profile opens", () => {
    expect(describeRemote({ ...base, kind: "wsl", distribution: "Ubuntu" })).toBe("WSL Ubuntu");
    expect(describeRemote({ ...base, kind: "wsl" })).toBe("WSL (default distribution)");
  });

  it("says nothing about a local profile, or one from before profiles had a kind", () => {
    expect(describeRemote(base)).toBeNull();
    expect(describeRemote({ ...base, kind: "local" })).toBeNull();
  });

  it("describes a remote profile by where it connects, not by a shell of this computer", () => {
    const ssh: Profile = { ...base, kind: "ssh", host: "build-box", shell: "/bin/zsh", workingDirectory: "/local" };
    expect(describeProfile(ssh)).toBe("ssh build-box");
    const wsl: Profile = { ...base, kind: "wsl", distribution: "Debian", shell: "/bin/zsh", workingDirectory: "~/code" };
    expect(describeProfile(wsl)).toBe("WSL Debian · ~/code");
  });
});
