import { describe, expect, it } from "vitest";
import { matchShortcut, shortcutLabel, tabPosition } from "../src/lib/shortcuts";

type Mods = Partial<Record<"meta" | "ctrl" | "shift" | "alt", boolean>>;

function press(key: string, modifiers: Mods, code?: string) {
  return {
    key,
    code,
    metaKey: modifiers.meta ?? false,
    ctrlKey: modifiers.ctrl ?? false,
    shiftKey: modifiers.shift ?? false,
    altKey: modifiers.alt ?? false,
  };
}

describe("application shortcuts on macOS", () => {
  it("uses ⌘", () => {
    expect(matchShortcut(press("r", { meta: true }), "macos")).toBe("restart");
    expect(matchShortcut(press("B", { meta: true }), "macos")).toBe("blocks");
    expect(matchShortcut(press(",", { meta: true }), "macos")).toBe("preferences");
    expect(matchShortcut(press("t", { meta: true }), "macos")).toBe("newTab");
    expect(matchShortcut(press("w", { meta: true }), "macos")).toBe("closePane");
    expect(matchShortcut(press("d", { meta: true }), "macos")).toBe("splitRight");
    expect(matchShortcut(press("D", { meta: true, shift: true }), "macos")).toBe("splitDown");
    expect(matchShortcut(press("f", { meta: true }), "macos")).toBe("find");
  });

  it("switches tabs with ⌘1…9 and ⇧⌘[ ⇧⌘]", () => {
    expect(matchShortcut(press("1", { meta: true }, "Digit1"), "macos")).toBe("tab1");
    expect(matchShortcut(press("9", { meta: true }, "Digit9"), "macos")).toBe("tab9");
    expect(matchShortcut(press("0", { meta: true }, "Digit0"), "macos")).toBeNull();
    expect(matchShortcut(press("{", { meta: true, shift: true }, "BracketLeft"), "macos")).toBe("prevTab");
    expect(matchShortcut(press("}", { meta: true, shift: true }, "BracketRight"), "macos")).toBe("nextTab");
  });

  it("moves between panes with ⌘⌥ and the arrows", () => {
    expect(matchShortcut(press("ArrowLeft", { meta: true, alt: true }), "macos")).toBe("focusLeft");
    expect(matchShortcut(press("ArrowDown", { meta: true, alt: true }), "macos")).toBe("focusDown");
    expect(matchShortcut(press("ArrowLeft", { alt: true }), "macos")).toBeNull();
  });

  it("leaves the shell's Ctrl keys alone", () => {
    for (const key of ["r", "b", "t", "w", "d", "f"]) {
      expect(matchShortcut(press(key, { ctrl: true }), "macos")).toBeNull();
    }
    expect(matchShortcut(press("r", { meta: true, ctrl: true }), "macos")).toBeNull();
  });
});

describe("application shortcuts on Windows and Linux", () => {
  it("never claims plain Ctrl keys, they belong to the shell", () => {
    for (const platform of ["linux", "windows"]) {
      for (const key of ["r", "b", "k", "t", "w", "d", "e", "f"]) {
        expect(matchShortcut(press(key, { ctrl: true }), platform), `${key} on ${platform}`).toBeNull();
      }
    }
  });

  it("uses Ctrl+Shift, whatever case the key arrives in", () => {
    expect(matchShortcut(press("R", { ctrl: true, shift: true }), "linux")).toBe("restart");
    expect(matchShortcut(press("b", { ctrl: true, shift: true }), "windows")).toBe("blocks");
    expect(matchShortcut(press("T", { ctrl: true, shift: true }), "linux")).toBe("newTab");
    expect(matchShortcut(press("W", { ctrl: true, shift: true }), "linux")).toBe("closePane");
    expect(matchShortcut(press("D", { ctrl: true, shift: true }), "linux")).toBe("splitRight");
    expect(matchShortcut(press("E", { ctrl: true, shift: true }), "linux")).toBe("splitDown");
    expect(matchShortcut(press("F", { ctrl: true, shift: true }), "linux")).toBe("find");
    expect(matchShortcut(press(",", { ctrl: true }), "linux")).toBe("preferences");
  });

  it("cycles tabs with Ctrl+Tab and picks them with Ctrl+Shift+digit (by code, the key is a symbol then)", () => {
    expect(matchShortcut(press("Tab", { ctrl: true }), "linux")).toBe("nextTab");
    expect(matchShortcut(press("Tab", { ctrl: true, shift: true }), "linux")).toBe("prevTab");
    expect(matchShortcut(press("!", { ctrl: true, shift: true }, "Digit1"), "windows")).toBe("tab1");
    expect(matchShortcut(press("(", { ctrl: true, shift: true }, "Digit9"), "windows")).toBe("tab9");
  });

  it("moves between panes with Ctrl+Alt and the arrows", () => {
    expect(matchShortcut(press("ArrowRight", { ctrl: true, alt: true }), "linux")).toBe("focusRight");
    expect(matchShortcut(press("ArrowUp", { ctrl: true, alt: true }), "windows")).toBe("focusUp");
    expect(matchShortcut(press("ArrowUp", { alt: true }), "linux")).toBeNull();
  });

  it("ignores ⌘ and other wrong modifiers", () => {
    expect(matchShortcut(press("r", { meta: true }), "linux")).toBeNull();
    expect(matchShortcut(press("r", { ctrl: true, shift: true, meta: true }), "linux")).toBeNull();
    expect(matchShortcut(press("r", { ctrl: true, shift: true, alt: true }), "linux")).toBeNull();
  });
});

describe("labels", () => {
  it("are spelled the way each platform spells them", () => {
    expect(shortcutLabel("restart", "macos")).toBe("⌘R");
    expect(shortcutLabel("restart", "linux")).toBe("Ctrl+Shift+R");
    expect(shortcutLabel("preferences", "macos")).toBe("⌘,");
    expect(shortcutLabel("preferences", "windows")).toBe("Ctrl+,");
    expect(shortcutLabel("newTab", "macos")).toBe("⌘T");
    expect(shortcutLabel("splitDown", "macos")).toBe("⇧⌘D");
    expect(shortcutLabel("splitDown", "linux")).toBe("Ctrl+Shift+E");
    expect(shortcutLabel("nextTab", "linux")).toBe("Ctrl+Tab");
    expect(shortcutLabel("focusLeft", "macos")).toBe("⌥⌘←");
    expect(shortcutLabel("focusDown", "windows")).toBe("Ctrl+Alt+↓");
    expect(shortcutLabel("tab3", "macos")).toBe("⌘3");
    expect(shortcutLabel("tab3", "linux")).toBe("Ctrl+Shift+3");
  });

  it("every shortcut that exists has a label", () => {
    for (const platform of ["macos", "linux"]) {
      for (const shortcut of ["preferences", "restart", "blocks", "newTab", "closePane", "splitRight", "splitDown", "find", "nextTab", "prevTab", "focusLeft", "focusRight", "focusUp", "focusDown", "tab1"] as const) {
        expect(shortcutLabel(shortcut, platform).length).toBeGreaterThan(1);
      }
    }
  });
});

describe("tab positions", () => {
  it("are zero-based", () => {
    expect(tabPosition("tab1")).toBe(0);
    expect(tabPosition("tab9")).toBe(8);
    expect(tabPosition("newTab")).toBeNull();
  });
});
