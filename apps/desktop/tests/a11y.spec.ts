/**
 * Automated accessibility checks (axe-core) of the screens a person uses: the palette, every section of Preferences,
 * the tab bar, the status bar and the toolbar. They find what a machine can find (names for controls, labels for
 * fields, roles, valid ARIA); contrast is checked separately against the themes (contrast.spec.ts), because there is
 * no layout or paint here.
 */
import axe from "axe-core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushSync, mount, unmount } from "svelte";

vi.mock("@tauri-apps/api/core", () => ({
  Channel: class {
    onmessage: (message: unknown) => void = () => {};
  },
  invoke: vi.fn(async (command: string) => {
    switch (command) {
      case "list_plugins":
        return { plugins: [], problems: [] };
      case "list_snippets":
      case "list_tasks":
      case "list_themes":
      case "detect_shells":
      case "list_ssh_hosts":
      case "list_wsl_distributions":
        return [];
      case "app_info":
        return { version: "1.0.0", platform: "macos", configPath: "/x", blurSupported: true };
      default:
        return null;
    }
  }),
}));

import AppearanceSection from "../src/lib/components/preferences/AppearanceSection.svelte";
import CommandPalette from "../src/lib/components/CommandPalette.svelte";
import GeneralSection from "../src/lib/components/preferences/GeneralSection.svelte";
import KeyboardSection from "../src/lib/components/preferences/KeyboardSection.svelte";
import PluginsSection from "../src/lib/components/preferences/PluginsSection.svelte";
import ProfilesSection from "../src/lib/components/preferences/ProfilesSection.svelte";
import SettingsModal from "../src/lib/components/SettingsModal.svelte";
import SnippetsSection from "../src/lib/components/preferences/SnippetsSection.svelte";
import StatusBar from "../src/lib/components/StatusBar.svelte";
import TabBar from "../src/lib/components/TabBar.svelte";
import TerminalSection from "../src/lib/components/preferences/TerminalSection.svelte";
import Toolbar from "../src/lib/components/Toolbar.svelte";
import { defaultSettings } from "../src/lib/settings";
import { pluginStore } from "../src/lib/stores/plugins.svelte";

class FakeResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}

let target: HTMLElement;
let component: ReturnType<typeof mount> | undefined;

beforeEach(async () => {
  // The window loads the plugins before anything else uses them.
  await pluginStore.load();
  vi.stubGlobal("ResizeObserver", FakeResizeObserver);
  target = document.createElement("div");
  document.body.append(target);
});

afterEach(() => {
  if (component) unmount(component);
  component = undefined;
  target.remove();
  vi.unstubAllGlobals();
});

/** The problems axe finds in what is on the page, as short sentences. */
async function violations(): Promise<string[]> {
  const result = await axe.run(target, {
    // Needs layout and paint, which this environment has none of; the themes are audited in contrast.spec.ts.
    rules: { "color-contrast": { enabled: false }, region: { enabled: false } },
    resultTypes: ["violations"],
  });
  return result.violations.flatMap((violation) =>
    violation.nodes.map((node) => `${violation.id}: ${violation.help} (${node.target.join(" ")}) ${node.failureSummary?.replace(/\s+/g, " ")}`),
  );
}

function show(Component: Parameters<typeof mount>[0], props: Record<string, unknown> = {}) {
  component = mount(Component, { target, props });
  flushSync();
}

const draft = () => ({ draft: defaultSettings() });

describe("Preferences", () => {
  const sections: [string, Parameters<typeof mount>[0]][] = [
    ["General", GeneralSection],
    ["Appearance", AppearanceSection],
    ["Terminal", TerminalSection],
    ["Keyboard", KeyboardSection],
    ["Profiles", ProfilesSection],
    ["Snippets", SnippetsSection],
    ["Plugins", PluginsSection],
  ];
  for (const [name, Section] of sections) {
    it(`the ${name} section has no violations`, async () => {
      show(Section, draft());
      await new Promise((resolve) => setTimeout(resolve, 20));

      expect(await violations()).toEqual([]);
    });
  }

  it("the dialog has no violations", async () => {
    show(SettingsModal);
    await new Promise((resolve) => setTimeout(resolve, 20));

    expect(await violations()).toEqual([]);
  });
});

describe("the window", () => {
  it("the command palette has no violations", async () => {
    show(CommandPalette);
    await new Promise((resolve) => setTimeout(resolve, 20));

    expect(await violations()).toEqual([]);
  });

  it("the tab bar has no violations", async () => {
    show(TabBar);

    expect(await violations()).toEqual([]);
  });

  it("the status bar has no violations", async () => {
    show(StatusBar);

    expect(await violations()).toEqual([]);
  });

  it("the toolbar has no violations", async () => {
    show(Toolbar);

    expect(await violations()).toEqual([]);
  });
});
