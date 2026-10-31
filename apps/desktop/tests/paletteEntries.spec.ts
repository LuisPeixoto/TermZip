import { describe, expect, it, vi } from "vitest";
import type { Profile } from "../src/lib/api";
import { defaultBindings } from "../src/lib/keybindings";
import { paletteEntries, type PaletteInput } from "../src/lib/paletteEntries";
import { highlight } from "../src/lib/fuzzy";
import { builtIn, builtInThemes } from "./helpers/themes";

function setup(extra: Partial<PaletteInput> = {}) {
  const commands = {
    run: vi.fn(),
    goToTab: vi.fn(),
    newTab: vi.fn(),
    useTheme: vi.fn(),
    toggleAppearance: vi.fn(),
    toggleBlur: vi.fn(),
    runPluginCommand: vi.fn(),
  };
  const input: PaletteInput = {
    platform: "macos",
    bindings: defaultBindings("macos"),
    themes: builtInThemes().map((theme) => ({ builtIn: true, theme })),
    profiles: [],
    tabs: ["zsh", "server"],
    currentThemeId: "termzip-dark",
    blurSupported: true,
    plugins: [],
    commands,
    ...extra,
  };
  return { entries: paletteEntries(input), commands };
}

const profile = (id: string, name: string): Profile => ({ id, name, shell: null, workingDirectory: null, env: {} });

describe("the palette's entries", () => {
  it("lists the actions with their shortcuts, but not the palette itself or tabs by number", () => {
    const { entries } = setup();
    const split = entries.find((entry) => entry.id === "action:splitRight");

    expect(split).toMatchObject({ title: "Split to the right", detail: "⌘D" });
    expect(entries.some((entry) => entry.id === "action:commandPalette")).toBe(false);
    expect(entries.some((entry) => /^action:tab\d/.test(entry.id))).toBe(false);
  });

  it("shows a changed shortcut and no shortcut for an unbound action", () => {
    const bindings = { ...defaultBindings("macos"), splitRight: "Shift+Cmd+E", find: "" };
    const { entries } = setup({ bindings });

    expect(entries.find((entry) => entry.id === "action:splitRight")?.detail).toBe("⇧⌘E");
    expect(entries.find((entry) => entry.id === "action:find")?.detail).toBe("");
  });

  it("has an entry for each open tab with its shortcut, up to nine", () => {
    const { entries } = setup();
    expect(entries.find((entry) => entry.id === "tab:2")).toMatchObject({ title: "Go to tab 2: server", detail: "⌘2" });

    const many = setup({ tabs: Array.from({ length: 12 }, (_, n) => `t${n}`) }).entries;
    expect(many.filter((entry) => entry.id.startsWith("tab:"))).toHaveLength(9);
  });

  it("offers a new tab for each profile only when there are profiles", () => {
    expect(setup().entries.some((entry) => entry.id.startsWith("new-tab:"))).toBe(false);

    const { entries, commands } = setup({ profiles: [profile("work", "Work")] });
    const work = entries.find((entry) => entry.id === "new-tab:work");
    expect(work?.title).toBe("New tab: Work");
    work?.run();
    expect(commands.newTab).toHaveBeenCalledWith("work");
    entries.find((entry) => entry.id === "new-tab:default")?.run();
    expect(commands.newTab).toHaveBeenCalledWith("default");
  });

  it("has an entry for every theme, marking the one in use", () => {
    const { entries, commands } = setup();
    const dracula = entries.find((entry) => entry.id === "theme:dracula");

    expect(dracula).toMatchObject({ title: "Theme: Dracula", detail: "dark" });
    expect(entries.find((entry) => entry.id === "theme:termzip-dark")?.detail).toBe("in use");
    expect(entries.find((entry) => entry.id === "theme:catppuccin-latte")?.detail).toBe("light");
    dracula?.run();
    expect(commands.useTheme).toHaveBeenCalledWith(builtIn("dracula"));
  });

  it("runs what each entry says", () => {
    const { entries, commands } = setup();

    entries.find((entry) => entry.id === "action:newTab")?.run();
    expect(commands.run).toHaveBeenCalledWith("newTab");
    entries.find((entry) => entry.id === "tab:1")?.run();
    expect(commands.goToTab).toHaveBeenCalledWith(0);
    entries.find((entry) => entry.id === "switch:appearance")?.run();
    expect(commands.toggleAppearance).toHaveBeenCalled();
  });

  it("offers the blur switch only where the system can blur", () => {
    expect(setup().entries.some((entry) => entry.id === "switch:blur")).toBe(true);
    expect(setup({ blurSupported: false }).entries.some((entry) => entry.id === "switch:blur")).toBe(false);
  });

  it("has an entry for each command of the running plugins", () => {
    const plugins = [
      { plugin: "demo", pluginName: "Demo plugin", id: "status", title: "Type git status" },
      { plugin: "demo", pluginName: "Demo plugin", id: "log", title: "Type git log" },
    ];
    const { entries, commands } = setup({ plugins });
    const typed = entries.find((entry) => entry.id === "plugin:demo:status");

    expect(typed).toMatchObject({ title: "Demo plugin: Type git status", detail: "plugin" });
    expect(entries.filter((entry) => entry.id.startsWith("plugin:"))).toHaveLength(2);
    typed?.run();
    expect(commands.runPluginCommand).toHaveBeenCalledWith("demo", "status");
    expect(setup().entries.some((entry) => entry.id.startsWith("plugin:"))).toBe(false);
  });

  it("gives every entry a unique id", () => {
    const { entries } = setup({ profiles: [profile("work", "Work")] });
    expect(new Set(entries.map((entry) => entry.id)).size).toBe(entries.length);
  });
});

describe("highlighting", () => {
  it("cuts the text at the matched stretches", () => {
    expect(highlight("New tab", [[0, 1], [4, 5]])).toEqual([
      { text: "N", hit: true },
      { text: "ew ", hit: false },
      { text: "t", hit: true },
      { text: "ab", hit: false },
    ]);
    expect(highlight("abc", [])).toEqual([{ text: "abc", hit: false }]);
    expect(highlight("abc", [[0, 3]])).toEqual([{ text: "abc", hit: true }]);
  });
});
