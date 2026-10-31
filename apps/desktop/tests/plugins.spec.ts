import { describe, expect, it } from "vitest";
import type { Plugin, PluginManifest } from "../src/lib/api";
import {
  describeAdds,
  describeAsk,
  describeSource,
  describeState,
  permissionChips,
  stateOf,
} from "../src/lib/plugins";

function manifest(extra: Partial<PluginManifest> = {}): PluginManifest {
  return {
    api: 1,
    id: "demo",
    name: "Demo",
    version: "1.0.0",
    description: "",
    permissions: [],
    commands: [],
    links: [],
    ...extra,
  };
}

function plugin(extra: Partial<Plugin> = {}, m: Partial<PluginManifest> = {}): Plugin {
  return {
    manifest: manifest(m),
    sha256: "0".repeat(64),
    size: 100,
    source: "local",
    enabled: false,
    changed: false,
    running: false,
    permissionNotes: [],
    ...extra,
  };
}

describe("permissions", () => {
  it("get a short label each, and the ones that read or act are marked", () => {
    const chips = permissionChips(manifest({ permissions: ["blocks", "output", "type", "notify", "open"] }));

    expect(chips.map((chip) => chip.label)).toEqual([
      "Sees your commands",
      "Sees what they print",
      "Types into the terminal",
      "Shows messages",
      "Opens web addresses",
    ]);
    expect(chips.filter((chip) => chip.sensitive).map((chip) => chip.permission)).toEqual(["output", "type", "open"]);
    expect(permissionChips(manifest())).toEqual([]);
  });
});

describe("what a plugin adds", () => {
  it("counts commands and patterns and says when it reacts to commands", () => {
    expect(describeAdds(manifest())).toBe("adds nothing");
    expect(describeAdds(manifest({ commands: [{ id: "a", title: "A" }] }))).toBe("1 command");
    expect(
      describeAdds(
        manifest({
          permissions: ["blocks"],
          commands: [
            { id: "a", title: "A" },
            { id: "b", title: "B" },
          ],
          links: [{ pattern: "#1", url: "https://x.test" }],
        }),
      ),
    ).toBe("2 commands · 1 link pattern · reacts to commands");
  });
});

describe("where a plugin came from", () => {
  it("says a file is not verified and a catalog plugin is", () => {
    expect(describeSource(plugin({ source: "local" }))).toMatchObject({ verified: false });
    expect(describeSource(plugin({ source: "catalog" }))).toMatchObject({ verified: true });
    expect(describeSource(plugin({ source: "local" })).label).toContain("not verified");
  });
});

describe("the state of a plugin", () => {
  it("tells running from stopped, changed and off", () => {
    expect(stateOf(plugin({ enabled: true, running: true }))).toBe("running");
    expect(stateOf(plugin({ enabled: true, running: false }))).toBe("stopped");
    expect(stateOf(plugin({ enabled: false, changed: true }))).toBe("changed");
    expect(stateOf(plugin())).toBe("off");
    // A changed file wins over everything else: it is the thing to explain.
    expect(stateOf(plugin({ enabled: true, running: true, changed: true }))).toBe("changed");
  });

  it("explains each one in a sentence", () => {
    expect(describeState("running")).toBe("On");
    expect(describeState("off")).toBe("Off");
    expect(describeState("stopped")).toContain("crashed three times");
    expect(describeState("changed")).toContain("changed");
  });
});

describe("the confirmation", () => {
  it("introduces the list of permissions, or says there are none", () => {
    expect(describeAsk(plugin({}, { permissions: ["notify"] }))).toBe("“Demo” asks to:");
    expect(describeAsk(plugin())).toContain("asks for no permission");
  });
});
