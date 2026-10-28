import { describe, expect, it } from "vitest";
import {
  ACTIONS,
  chordOf,
  chordSafety,
  conflictFor,
  defaultBindings,
  formatChord,
  matchChord,
  normalizeChord,
  resolveBindings,
} from "../src/lib/keybindings";
import { matchShortcut, shortcutLabel } from "../src/lib/shortcuts";

type Mods = Partial<Record<"meta" | "ctrl" | "shift" | "alt", boolean>>;

function press(key: string, modifiers: Mods = {}, code?: string) {
  return {
    key,
    code,
    metaKey: modifiers.meta ?? false,
    ctrlKey: modifiers.ctrl ?? false,
    shiftKey: modifiers.shift ?? false,
    altKey: modifiers.alt ?? false,
  };
}

describe("the shortcuts that ship", () => {
  for (const platform of ["macos", "linux", "windows"]) {
    it(`give every action its own valid chord on ${platform}`, () => {
      const bindings = defaultBindings(platform);
      const chords = ACTIONS.map((action) => bindings[action.id]);

      expect(chords.every((chord) => chord !== "" && normalizeChord(chord) === chord)).toBe(true);
      expect(new Set(chords).size).toBe(chords.length);
    });

    it(`never claim a chord the shell or the terminal needs on ${platform}`, () => {
      const bindings = defaultBindings(platform);
      for (const action of ACTIONS) {
        expect(chordSafety(bindings[action.id], platform).level, `${action.id} ${bindings[action.id]}`).toBe("ok");
      }
    });
  }
});

describe("what a key press is", () => {
  it("writes modifiers in a fixed order and letters as typed", () => {
    expect(chordOf(press("d", { meta: true, shift: true }))).toBe("Shift+Cmd+D");
    expect(chordOf(press("T", { ctrl: true, shift: true, alt: true }))).toBe("Ctrl+Alt+Shift+T");
  });

  it("follows the character on other layouts, so a letter shortcut is the letter on the key cap", () => {
    // On Dvorak the key that types "t" sits where QWERTY has "y".
    expect(chordOf(press("t", { meta: true }, "KeyY"))).toBe("Cmd+T");
    // A layout with no Latin letters falls back to the physical key.
    expect(chordOf(press("е", { meta: true }, "KeyT"))).toBe("Cmd+T");
  });

  it("follows the physical key for digits and brackets, because Shift changes what they type", () => {
    expect(chordOf(press("!", { ctrl: true, shift: true }, "Digit1"))).toBe("Ctrl+Shift+1");
    expect(chordOf(press("{", { meta: true, shift: true }, "BracketLeft"))).toBe("Shift+Cmd+BracketLeft");
    expect(chordOf(press(",", { meta: true }, "Comma"))).toBe("Cmd+Comma");
  });

  it("copes with events that carry no code", () => {
    expect(chordOf(press("ArrowLeft", { alt: true }))).toBe("Alt+Left");
    expect(chordOf(press("Tab", { ctrl: true }))).toBe("Ctrl+Tab");
    expect(chordOf(press(",", { ctrl: true }))).toBe("Ctrl+Comma");
    expect(chordOf(press("F5"))).toBe("F5");
  });

  it("is nothing for a modifier on its own", () => {
    for (const key of ["Shift", "Control", "Alt", "Meta", "AltGraph", "CapsLock"]) {
      expect(chordOf(press(key, { shift: true }))).toBeNull();
    }
  });
});

describe("customized shortcuts", () => {
  it("replace the default of the action they name and leave the others", () => {
    const bindings = resolveBindings({ splitRight: "Cmd+Shift+E" }, "macos");

    expect(bindings.splitRight).toBe("Shift+Cmd+E");
    expect(bindings.splitDown).toBe("Shift+Cmd+D");
    expect(matchChord(press("e", { meta: true, shift: true }), bindings)).toBe("splitRight");
    // The old chord of the action is free again.
    expect(matchChord(press("d", { meta: true }), bindings)).toBeNull();
  });

  it("can unbind an action with an empty chord", () => {
    const bindings = resolveBindings({ find: "" }, "linux");

    expect(bindings.find).toBe("");
    expect(matchChord(press("F", { ctrl: true, shift: true }), bindings)).toBeNull();
    expect(formatChord(bindings.find, "linux")).toBe("");
  });

  it("ignores actions that do not exist and chords that are not valid", () => {
    const bindings = resolveBindings({ teleport: "Cmd+9", newTab: "T", closePane: "Shift+W" }, "macos");

    expect(bindings).not.toHaveProperty("teleport");
    // A chord that cannot be a shortcut leaves the action without one, never with the bare key.
    expect(bindings.newTab).toBe("");
    expect(bindings.closePane).toBe("");
  });

  it("never lets one key do two things when a hand-edited file repeats a chord", () => {
    const bindings = resolveBindings({ closePane: "Cmd+T" }, "macos");

    // `newTab` comes first in the list of actions and keeps the chord.
    expect(bindings.newTab).toBe("Cmd+T");
    expect(bindings.closePane).toBe("");
  });

  it("go through the same functions the interface uses", () => {
    const overrides = { splitRight: "Cmd+Shift+E", restart: "Ctrl+Alt+R" };

    expect(matchShortcut(press("e", { meta: true, shift: true }), "macos", overrides)).toBe("splitRight");
    expect(matchShortcut(press("d", { meta: true }), "macos", overrides)).toBeNull();
    expect(shortcutLabel("splitRight", "macos", overrides)).toBe("⇧⌘E");
    expect(shortcutLabel("restart", "macos", overrides)).toBe("⌃⌥R");
    expect(shortcutLabel("restart", "linux", overrides)).toBe("Ctrl+Alt+R");
    expect(shortcutLabel("restart", "linux")).toBe("Ctrl+Shift+R");
  });
});

describe("chord spelling", () => {
  // The same cases as the backend's tests (keybindings.rs), so the two cannot drift apart.
  it("is canonical", () => {
    expect(normalizeChord("cmd+shift+d")).toBe("Shift+Cmd+D");
    expect(normalizeChord("Shift + Ctrl + t")).toBe("Ctrl+Shift+T");
    expect(normalizeChord("Option+Command+Left")).toBe("Alt+Cmd+Left");
    expect(normalizeChord("ctrl+tab")).toBe("Ctrl+Tab");
    expect(normalizeChord("Cmd+bracketleft")).toBe("Cmd+BracketLeft");
    expect(normalizeChord("Ctrl+Shift+3")).toBe("Ctrl+Shift+3");
    expect(normalizeChord("F5")).toBe("F5");
    expect(normalizeChord("shift+f12")).toBe("Shift+F12");
  });

  it("refuses what would break typing or is malformed", () => {
    for (const bad of [
      "", " ", "+", "Ctrl+", "+D", "Ctrl++D", "Ctrl+Ctrl+D", "Ctrl+D+E", "Ctrl+Hyper+D", "Ctrl+Escape", "Ctrl+F25",
      "Ctrl+F0", "Cmd+,", "D", "Shift+D", "Shift+3", "Space", "Shift+Tab", "Enter",
    ]) {
      expect(normalizeChord(bad), JSON.stringify(bad)).toBeNull();
    }
  });
});

describe("showing a chord", () => {
  it("uses the symbols of each platform", () => {
    expect(formatChord("Ctrl+Alt+Shift+Cmd+K", "macos")).toBe("⌃⌥⇧⌘K");
    expect(formatChord("Alt+Cmd+Left", "macos")).toBe("⌥⌘←");
    expect(formatChord("Shift+Cmd+BracketRight", "macos")).toBe("⇧⌘]");
    expect(formatChord("Ctrl+Shift+Tab", "linux")).toBe("Ctrl+Shift+Tab");
    expect(formatChord("Ctrl+Alt+Down", "windows")).toBe("Ctrl+Alt+↓");
    expect(formatChord("Cmd+Comma", "macos")).toBe("⌘,");
    expect(formatChord("Ctrl+Comma", "linux")).toBe("Ctrl+,");
    expect(formatChord("", "macos")).toBe("");
  });
});

describe("is a chord a good idea", () => {
  it("refuses what takes over typing", () => {
    for (const chord of ["D", "Shift+D", "Space"]) {
      expect(chordSafety(chord, "macos").level, chord).toBe("refuse");
    }
  });

  it("refuses the chords the terminal keeps for copy, paste and select all", () => {
    expect(chordSafety("Cmd+C", "macos").level).toBe("refuse");
    expect(chordSafety("Cmd+V", "macos").level).toBe("refuse");
    expect(chordSafety("Ctrl+Shift+A", "linux").level).toBe("refuse");
    expect(chordSafety("Ctrl+Shift+C", "windows").level).toBe("refuse");
  });

  it("warns about the keys a shell uses, and says which", () => {
    const control = chordSafety("Ctrl+R", "linux");
    expect(control.level).toBe("warn");
    expect("message" in control && control.message).toContain("Ctrl+R");
    expect(chordSafety("Ctrl+K", "macos").level).toBe("warn");
    expect(chordSafety("Alt+B", "linux").level).toBe("warn");
    expect(chordSafety("Alt+B", "macos").level).toBe("warn");
  });

  it("lets through the chords nobody types", () => {
    expect(chordSafety("Cmd+K", "macos").level).toBe("ok");
    expect(chordSafety("Ctrl+Shift+K", "linux").level).toBe("ok");
    expect(chordSafety("Ctrl+Alt+K", "linux").level).toBe("ok");
    expect(chordSafety("Ctrl+Tab", "windows").level).toBe("ok");
    expect(chordSafety("F5", "linux").level).toBe("ok");
  });
});

describe("conflicts", () => {
  it("names the action that already has a chord", () => {
    const bindings = defaultBindings("macos");

    expect(conflictFor(bindings, "Cmd+T", "closePane")?.id).toBe("newTab");
    expect(conflictFor(bindings, "Cmd+T", "newTab")).toBeNull();
    expect(conflictFor(bindings, "Cmd+K", "newTab")).toBeNull();
    expect(conflictFor(bindings, "", "newTab")).toBeNull();
  });
});
