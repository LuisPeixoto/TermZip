import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { flushSync } from "svelte";

vi.mock("@tauri-apps/api/core", () => ({
  Channel: class {
    onmessage: (message: unknown) => void = () => {};
  },
  invoke: vi.fn(),
}));

import type { Workspace } from "../src/lib/api";
import { leaves } from "../src/lib/layout";
import { persistWorkspace } from "../src/lib/stores/persist.svelte";
import { WorkspaceStore } from "../src/lib/stores/workspace.svelte";

let store: WorkspaceStore;

beforeEach(() => {
  store = new WorkspaceStore();
});

/** Two tabs: the second has a split with a profile and folders. */
function build(): WorkspaceStore {
  const first = store.activePane;
  first.cwd = "/srv/one";
  const second = store.newTab()!;
  second.active.cwd = "/srv/two";
  second.active.profileId = "work";
  const right = store.splitActive("row")!;
  right.cwd = "/srv/two/api";
  store.renameTab(second.id, "backend");
  second.focus(second.paneList[0].id);
  return store;
}

describe("snapshot", () => {
  it("records structure, folders, profiles and what has the keyboard, never output", () => {
    build();
    const saved = store.snapshot();

    expect(saved.version).toBe(1);
    expect(saved.activeTab).toBe(1);
    expect(saved.tabs).toHaveLength(2);
    expect(saved.tabs[0]).toMatchObject({ title: null, activePane: 0, root: { kind: "leaf", cwd: "/srv/one" } });
    const second = saved.tabs[1];
    expect(second.title).toBe("backend");
    expect(second.activePane).toBe(0);
    expect(second.root).toMatchObject({ kind: "split", direction: "row" });
    expect(JSON.stringify(saved)).not.toMatch(/blocks|output|session/i);
  });

  it("keeps the folder a pane was restored into until its shell reports another", () => {
    const pane = store.activePane;
    pane.restoreDirectory = "/restored";
    expect(store.snapshot().tabs[0].root).toMatchObject({ cwd: "/restored" });
    pane.cwd = "/now";
    expect(store.snapshot().tabs[0].root).toMatchObject({ cwd: "/now" });
  });
});

describe("restore", () => {
  it("brings the layout, names, profiles and folders back, in new panes", () => {
    build();
    const saved = store.snapshot();
    const before = store.tabs.flatMap((tab) => tab.paneList.map((pane) => pane.id));

    const fresh = new WorkspaceStore();
    fresh.restore(saved);

    expect(fresh.loaded).toBe(true);
    expect(fresh.tabs).toHaveLength(2);
    expect(fresh.activeTab).toBe(fresh.tabs[1]);
    expect(fresh.tabs[1].name).toBe("backend");
    expect(leaves(fresh.tabs[1].layout)).toHaveLength(2);

    const [left, right] = fresh.tabs[1].paneList;
    expect(left.profileId).toBe("work");
    expect(left.restoreDirectory).toBe("/srv/two");
    expect(right.restoreDirectory).toBe("/srv/two/api");
    expect(fresh.tabs[1].active).toBe(left);

    const after = fresh.tabs.flatMap((tab) => tab.paneList.map((pane) => pane.id));
    expect(after.some((id) => before.includes(id))).toBe(false);
    expect(fresh.snapshot()).toEqual(saved);
  });

  it("keeps the fresh tab when nothing was saved, and still counts as loaded", () => {
    const fresh = new WorkspaceStore();
    expect(fresh.loaded).toBe(false);
    const only = fresh.tabs[0];

    fresh.restore(null);
    expect(fresh.loaded).toBe(true);
    expect(fresh.tabs).toEqual([only]);

    const other = new WorkspaceStore();
    other.restore({ version: 1, activeTab: 0, tabs: [] });
    expect(other.tabs).toHaveLength(1);
    expect(other.loaded).toBe(true);
  });

  it("clamps an active tab that points past the end", () => {
    build();
    const saved: Workspace = { ...store.snapshot(), activeTab: 99 };
    const fresh = new WorkspaceStore();
    fresh.restore(saved);
    expect(fresh.activeTab).toBe(fresh.tabs.at(-1));
  });
});

describe("persistence", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("writes once, after the changes settle, and not at all when nothing changed", () => {
    const saved: Workspace[] = [];
    const persistence = persistWorkspace(store, async (workspace) => void saved.push(workspace), 500);
    flushSync();
    vi.advanceTimersByTime(2000);
    expect(saved).toHaveLength(0);

    store.activePane.cwd = "/a";
    flushSync();
    store.activePane.cwd = "/b";
    flushSync();
    store.newTab();
    flushSync();
    vi.advanceTimersByTime(400);
    expect(saved).toHaveLength(0);

    vi.advanceTimersByTime(200);
    expect(saved).toHaveLength(1);
    expect(saved[0].tabs).toHaveLength(2);
    expect(saved[0].tabs[0].root).toMatchObject({ cwd: "/b" });

    persistence.stop();
  });

  it("flushes at once when the window is closing", () => {
    const saved: Workspace[] = [];
    const persistence = persistWorkspace(store, async (workspace) => void saved.push(workspace), 500);
    flushSync();

    store.activePane.cwd = "/closing";
    flushSync();
    persistence.flush();
    expect(saved).toHaveLength(1);

    persistence.flush();
    expect(saved).toHaveLength(1);
    vi.advanceTimersByTime(2000);
    expect(saved).toHaveLength(1);
    persistence.stop();
  });

  it("survives a failing save", () => {
    const persistence = persistWorkspace(store, () => Promise.reject(new Error("disk full")), 100);
    flushSync();
    store.activePane.cwd = "/x";
    flushSync();
    expect(() => vi.advanceTimersByTime(200)).not.toThrow();
    persistence.stop();
  });
});
