import { describe, expect, it } from "vitest";
import { matchShortcut, shortcutLabel } from "../src/lib/shortcuts";

function press(key: string, modifiers: Partial<Record<"meta" | "ctrl" | "shift" | "alt", boolean>>) {
  return {
    key,
    metaKey: modifiers.meta ?? false,
    ctrlKey: modifiers.ctrl ?? false,
    shiftKey: modifiers.shift ?? false,
    altKey: modifiers.alt ?? false,
  };
}

describe("application shortcuts", () => {
  it("uses ⌘ on macOS", () => {
    expect(matchShortcut(press("r", { meta: true }), "macos")).toBe("restart");
    expect(matchShortcut(press("B", { meta: true }), "macos")).toBe("blocks");
    expect(matchShortcut(press(",", { meta: true }), "macos")).toBe("preferences");
  });

  it("leaves the shell's Ctrl keys alone on macOS", () => {
    expect(matchShortcut(press("r", { ctrl: true }), "macos")).toBeNull();
    expect(matchShortcut(press("b", { ctrl: true }), "macos")).toBeNull();
  });

  it("never claims plain Ctrl+R or Ctrl+B elsewhere, they belong to the shell", () => {
    for (const platform of ["linux", "windows"]) {
      expect(matchShortcut(press("r", { ctrl: true }), platform)).toBeNull();
      expect(matchShortcut(press("b", { ctrl: true }), platform)).toBeNull();
      expect(matchShortcut(press("k", { ctrl: true }), platform)).toBeNull();
    }
  });

  it("uses Ctrl+Shift elsewhere, whatever case the key arrives in", () => {
    expect(matchShortcut(press("R", { ctrl: true, shift: true }), "linux")).toBe("restart");
    expect(matchShortcut(press("b", { ctrl: true, shift: true }), "windows")).toBe("blocks");
    expect(matchShortcut(press(",", { ctrl: true }), "linux")).toBe("preferences");
  });

  it("ignores chords with Alt or the wrong modifiers", () => {
    expect(matchShortcut(press("r", { meta: true, alt: true }), "macos")).toBeNull();
    expect(matchShortcut(press("r", { meta: true, shift: true }), "macos")).toBeNull();
    expect(matchShortcut(press("r", { meta: true }), "linux")).toBeNull();
  });

  it("writes labels the way each platform spells them", () => {
    expect(shortcutLabel("restart", "macos")).toBe("⌘R");
    expect(shortcutLabel("restart", "linux")).toBe("Ctrl+Shift+R");
    expect(shortcutLabel("preferences", "macos")).toBe("⌘,");
    expect(shortcutLabel("preferences", "windows")).toBe("Ctrl+,");
  });
});
