import { describe, expect, it } from "vitest";
import { ACTIONS, defaultBindings, resolveBindings } from "../src/lib/keybindings";
import { assignChord, clearChord, filterActions, isCustomized, resetChord } from "../src/lib/shortcutEdit";

describe("giving an action a shortcut", () => {
  it("keeps only what differs from the default", () => {
    const { overrides, displaced } = assignChord({}, "macos", "splitRight", "Shift+Cmd+E");

    expect(overrides).toEqual({ splitRight: "Shift+Cmd+E" });
    expect(displaced).toBeNull();
    expect(resolveBindings(overrides, "macos").splitRight).toBe("Shift+Cmd+E");
  });

  it("drops the override again when the chord is the default one", () => {
    const first = assignChord({}, "macos", "splitRight", "Shift+Cmd+E").overrides;
    const back = assignChord(first, "macos", "splitRight", "Cmd+D").overrides;

    expect(back).toEqual({});
  });

  it("takes the chord from whoever had it, and says who", () => {
    const { overrides, displaced } = assignChord({}, "macos", "closePane", "Cmd+T");

    expect(displaced?.id).toBe("newTab");
    const bindings = resolveBindings(overrides, "macos");
    expect(bindings.closePane).toBe("Cmd+T");
    expect(bindings.newTab).toBe("");
    // Nothing else moved.
    expect(bindings.find).toBe(defaultBindings("macos").find);
  });

  it("never leaves two actions on one chord, however the edits come", () => {
    let overrides = {};
    for (const [action, chord] of [
      ["find", "Cmd+K"],
      ["blocks", "Cmd+K"],
      ["restart", "Cmd+K"],
      ["newTab", "Cmd+F"],
    ] as const) {
      overrides = assignChord(overrides, "macos", action, chord).overrides;
    }
    const chords = ACTIONS.map((action) => resolveBindings(overrides, "macos")[action.id]).filter(Boolean);

    expect(new Set(chords).size).toBe(chords.length);
    expect(resolveBindings(overrides, "macos").restart).toBe("Cmd+K");
  });
});

describe("taking a shortcut away", () => {
  it("leaves the action without one", () => {
    const overrides = clearChord({}, "linux", "find");

    expect(overrides).toEqual({ find: "" });
    expect(resolveBindings(overrides, "linux").find).toBe("");
  });

  it("is already the default for an action that has none", () => {
    // Clearing twice does not pile anything up.
    expect(clearChord(clearChord({}, "linux", "find"), "linux", "find")).toEqual({ find: "" });
  });
});

describe("going back to the default", () => {
  it("restores the default chord", () => {
    const moved = assignChord({}, "macos", "splitRight", "Shift+Cmd+E").overrides;
    const { overrides, displaced } = resetChord(moved, "macos", "splitRight");

    expect(overrides).toEqual({});
    expect(displaced).toBeNull();
  });

  it("takes the default chord back from an action that has since taken it", () => {
    // `closePane` took ⌘T from `newTab`; resetting `newTab` takes it back and leaves `closePane` bare.
    const taken = assignChord({}, "macos", "closePane", "Cmd+T").overrides;
    const { overrides, displaced } = resetChord(taken, "macos", "newTab");

    expect(displaced?.id).toBe("closePane");
    const bindings = resolveBindings(overrides, "macos");
    expect(bindings.newTab).toBe("Cmd+T");
    expect(bindings.closePane).toBe("");
  });
});

describe("what is customized", () => {
  it("is anything that is not the default", () => {
    const overrides = assignChord({}, "linux", "find", "Ctrl+Alt+F").overrides;

    expect(isCustomized(overrides, "linux", "find")).toBe(true);
    expect(isCustomized(overrides, "linux", "newTab")).toBe(false);
    expect(isCustomized(clearChord({}, "linux", "find"), "linux", "find")).toBe(true);
  });
});

describe("searching the actions", () => {
  it("matches every word, in any order, ignoring case", () => {
    const labels = filterActions(ACTIONS, "SPLIT right").map((action) => action.id);
    expect(labels).toEqual(["splitRight"]);
    expect(filterActions(ACTIONS, "pane").length).toBeGreaterThan(3);
    expect(filterActions(ACTIONS, "  ").length).toBe(ACTIONS.length);
    expect(filterActions(ACTIONS, "zzz")).toEqual([]);
  });
});
