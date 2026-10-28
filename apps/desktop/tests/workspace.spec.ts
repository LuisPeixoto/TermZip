import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  Channel: class {
    onmessage: (message: unknown) => void = () => {};
  },
  invoke: vi.fn(),
}));

import { WorkspaceStore } from "../src/lib/stores/workspace.svelte";

let store: WorkspaceStore;

beforeEach(() => {
  store = new WorkspaceStore();
});

const ids = () => store.tabs.map((tab) => tab.id);

describe("tabs", () => {
  it("starts with one tab and an active pane", () => {
    expect(store.tabs).toHaveLength(1);
    expect(store.activeTab).toBe(store.tabs[0]);
    expect(store.activePane.id).toBe(store.tabs[0].paneList[0].id);
  });

  it("opens a tab and switches to it", () => {
    const tab = store.newTab();
    expect(store.tabs).toHaveLength(2);
    expect(store.activeId).toBe(tab.id);
  });

  it("starts a new tab in the folder of the pane you were in", () => {
    store.activePane.cwd = "/srv/app";
    const tab = store.newTab();
    expect(tab.active.restoreDirectory).toBe("/srv/app");
    expect(tab.active.profileId).toBeNull();
  });

  it("starts a tab with an explicit profile in that profile's own folder", () => {
    store.activePane.cwd = "/srv/app";
    const tab = store.newTab("work");
    expect(tab.active.profileId).toBe("work");
    expect(tab.active.restoreDirectory).toBeNull();
  });

  it("closing the active tab activates its neighbor", () => {
    const [first] = store.tabs;
    const second = store.newTab();
    const third = store.newTab();

    store.activate(second.id);
    store.closeTab(second.id);
    expect(store.activeId).toBe(third.id);

    store.closeTab(third.id);
    expect(store.activeId).toBe(first.id);
  });

  it("closing another tab keeps the active one", () => {
    const [first] = store.tabs;
    const second = store.newTab();
    store.closeTab(first.id);
    expect(store.activeId).toBe(second.id);
  });

  it("never leaves the window without a tab", () => {
    const [only] = store.tabs;
    store.closeTab(only.id);
    expect(store.tabs).toHaveLength(1);
    expect(store.tabs[0].id).not.toBe(only.id);
    expect(store.activeId).toBe(store.tabs[0].id);
  });

  it("ignores ids and positions that do not exist", () => {
    const before = ids();
    store.closeTab("nope");
    store.activate("nope");
    store.activateAt(9);
    store.moveTab("nope", 0);
    store.renameTab("nope", "x");
    expect(ids()).toEqual(before);
    expect(store.activeId).toBe(before[0]);
  });

  it("switches by position and cycles around the ends", () => {
    const [a] = store.tabs;
    const b = store.newTab();
    const c = store.newTab();

    store.activateAt(0);
    expect(store.activeId).toBe(a.id);
    store.cycle(-1);
    expect(store.activeId).toBe(c.id);
    store.cycle(1);
    expect(store.activeId).toBe(a.id);
    store.cycle(1);
    expect(store.activeId).toBe(b.id);
  });

  it("reorders and clamps to the ends", () => {
    const [a] = store.tabs;
    const b = store.newTab();
    const c = store.newTab();

    store.moveTab(a.id, 2);
    expect(ids()).toEqual([b.id, c.id, a.id]);
    store.moveTab(a.id, -5);
    expect(ids()).toEqual([a.id, b.id, c.id]);
    store.moveTab(c.id, 99);
    expect(ids()).toEqual([a.id, b.id, c.id]);
  });
});

describe("tab titles", () => {
  it("prefers a name the user gave, then the shell's title, then the folder, then the shell", () => {
    const tab = store.activeTab;
    expect(tab.title).toBe("shell");

    tab.active.setSession({ id: 1, shell: "zsh", integrated: true });
    expect(tab.title).toBe("zsh");

    tab.active.cwd = "/Users/me/project";
    expect(tab.title).toBe("project");

    tab.active.title = "vim notes.md";
    expect(tab.title).toBe("vim notes.md");

    store.renameTab(tab.id, "  build  ");
    expect(tab.title).toBe("build");
  });

  it("goes back to following the shell when the name is cleared", () => {
    const tab = store.activeTab;
    tab.active.cwd = "/tmp/work";
    store.renameTab(tab.id, "custom");
    store.renameTab(tab.id, "   ");
    expect(tab.name).toBeNull();
    expect(tab.title).toBe("work");
  });

  it("bounds a name", () => {
    const tab = store.activeTab;
    store.renameTab(tab.id, "n".repeat(500));
    expect(tab.name).toHaveLength(64);
  });
});

describe("panes", () => {
  it("splits the active pane into the same profile and folder, and gives it the keyboard", () => {
    const first = store.activePane;
    first.profileId = "work";
    first.cwd = "/srv/app";

    const second = store.splitActive("row");

    expect(second).not.toBeNull();
    expect(second!.profileId).toBe("work");
    expect(second!.restoreDirectory).toBe("/srv/app");
    expect(store.activePane).toBe(second);
    expect(store.activeTab.paneList.map((p) => p.id)).toEqual([first.id, second!.id]);
  });

  it("keeps the existing pane objects when the layout changes, so their shells keep running", () => {
    const tab = store.activeTab;
    const first = tab.active;
    store.splitActive("row");
    store.splitActive("column");
    expect(tab.pane(first.id)).toBe(first);
    expect(tab.paneList).toContain(first);
  });

  it("stops at the pane limit", () => {
    let made = 1;
    while (store.splitActive(made % 2 ? "row" : "column") !== null) {
      made += 1;
      store.activeTab.focus(store.activeTab.paneList[0].id); // spread them out so only the count limits
    }
    expect(store.activeTab.paneList.length).toBeLessThanOrEqual(16);
    expect(store.splitActive("row")).toBeNull();
  });

  it("closing a pane gives the keyboard to the one before it", () => {
    const [a] = store.activeTab.paneList;
    const b = store.splitActive("row")!;
    const c = store.splitActive("row")!;

    expect(store.activeTab.closePane(c.id)).toBe(true);
    expect(store.activePane).toBe(b);
    expect(store.activeTab.pane(c.id)).toBeUndefined();

    expect(store.activeTab.closePane(a.id)).toBe(true);
    expect(store.activePane).toBe(b);
  });

  it("closing the only pane of a tab closes the tab", () => {
    const [first] = store.tabs;
    const second = store.newTab();
    expect(second.closePane(second.active.id)).toBe(false);

    store.closeActivePane();
    expect(store.tabs).toEqual([first]);
  });

  it("closing one of several panes leaves the tab open", () => {
    store.splitActive("row");
    const before = store.tabs.length;
    store.closeActivePane();
    expect(store.tabs).toHaveLength(before);
    expect(store.activeTab.paneList).toHaveLength(1);
  });

  it("moves the keyboard to a neighbor by geometry and stops at the edge", () => {
    const left = store.activePane;
    const right = store.splitActive("row")!;
    const below = store.splitActive("column")!;

    expect(store.activeTab.focusSide("up")).toBe(true);
    expect(store.activePane).toBe(right);
    expect(store.activeTab.focusSide("left")).toBe(true);
    expect(store.activePane).toBe(left);
    expect(store.activeTab.focusSide("left")).toBe(false);
    expect(store.activeTab.focusSide("right")).toBe(true);
    expect([right, below]).toContain(store.activePane);
  });

  it("resizes a split within the allowed range", () => {
    store.splitActive("row");
    const layout = store.activeTab.layout;
    if (layout.kind !== "split") throw new Error("expected a split");
    store.activeTab.setRatio(layout.id, 0.7);
    expect(store.activeTab.layout.kind === "split" && store.activeTab.layout.ratio).toBe(0.7);
    store.activeTab.setRatio(layout.id, 5);
    expect(store.activeTab.layout.kind === "split" && store.activeTab.layout.ratio).toBe(0.9);
  });
});
