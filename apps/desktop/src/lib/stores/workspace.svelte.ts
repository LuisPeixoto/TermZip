import { newId } from "../ids";
import type { SplitDirection, Workspace, WorkspaceTab } from "../api";
import * as tree from "../layout";
import { leaf, leaves, type Layout, type Side } from "../layout";
import { basename } from "../paths";
import { Pane } from "./pane.svelte";

/** The most tabs a saved workspace may hold (`workspace.rs` refuses more, and the whole file with it). */
export const MAX_TABS = 32;

/** One tab: a layout of panes and which of them has the keyboard. */
export class Tab {
  readonly id: string;
  /** A name the user gave the tab; `null` follows the shell. */
  name = $state<string | null>(null);
  layout = $state.raw<Layout>(leaf(""));
  activePaneId = $state("");
  // Looked up by id from the layout, which is what changes; panes are added before they are placed.
  private panes = new Map<string, Pane>();

  constructor(id: string, first: Pane) {
    this.id = id;
    this.panes.set(first.id, first);
    this.layout = leaf(first.id);
    this.activePaneId = first.id;
  }

  /** Replaces the layout with an existing set of panes, for restoring a saved tab. */
  adopt(layout: Layout, panes: Pane[], activeIndex: number): void {
    this.panes = new Map(panes.map((pane) => [pane.id, pane]));
    this.layout = layout;
    this.activePaneId = (panes[activeIndex] ?? panes[0]).id;
  }

  pane(id: string): Pane | undefined {
    return this.panes.get(id);
  }

  /** The pane that has the keyboard in this tab. */
  get active(): Pane {
    return this.panes.get(this.activePaneId) ?? [...this.panes.values()][0];
  }

  /** Panes in layout order. */
  get paneList(): Pane[] {
    return leaves(this.layout).flatMap((id) => {
      const pane = this.panes.get(id);
      return pane ? [pane] : [];
    });
  }

  /**
   * Puts a new pane beside the active one, in the same profile and folder, and gives it the keyboard.
   * `null` when the tab is at its limit of panes or nesting.
   */
  split(direction: SplitDirection, before = false): Pane | null {
    if (!tree.canSplit(this.layout, this.activePaneId)) return null;
    const source = this.active;
    const pane = new Pane(newId(), source.profileId, source.cwd);
    this.panes.set(pane.id, pane);
    this.layout = tree.split(this.layout, this.activePaneId, direction, pane.id, newId(), before);
    this.activePaneId = pane.id;
    return pane;
  }

  /** Closes a pane and gives its space to its sibling. `false` when it is the only pane. */
  closePane(id: string): boolean {
    const order = leaves(this.layout);
    const index = order.indexOf(id);
    if (index < 0 || order.length <= 1) return false;
    const next = tree.close(this.layout, id);
    if (!next) return false;
    if (this.activePaneId === id) {
      this.activePaneId = order[index - 1] ?? order[index + 1];
    }
    this.layout = next;
    this.panes.delete(id);
    return true;
  }

  focus(id: string): void {
    if (this.panes.has(id)) this.activePaneId = id;
  }

  /** Moves the keyboard to the pane next to the active one. `false` at an edge. */
  focusSide(side: Side): boolean {
    const target = tree.neighbor(this.layout, this.activePaneId, side);
    if (!target) return false;
    this.activePaneId = target;
    return true;
  }

  setRatio(splitId: string, ratio: number): void {
    this.layout = tree.setRatio(this.layout, splitId, ratio);
  }

  /** What the tab shows: its name, else the shell's title, else the folder, else the shell. */
  get title(): string {
    const pane = this.active;
    return this.name ?? pane.title ?? basename(pane.cwd) ?? pane.session?.shell ?? "shell";
  }
}

/** The tabs of the window. There is always at least one. */
export class WorkspaceStore {
  tabs = $state.raw<Tab[]>([]);
  activeId = $state("");
  /** False until the saved layout (if any) is in; nothing is shown before, or a shell would start for nothing. */
  loaded = $state(false);

  constructor() {
    this.newTab();
  }

  /** The layout as it is saved: structure only, with the folder each shell is in. */
  snapshot(): Workspace {
    const tabs: WorkspaceTab[] = this.tabs.map((tab) => ({
      title: tab.name,
      activePane: Math.max(0, leaves(tab.layout).indexOf(tab.activePaneId)),
      root: tree.toNode(tab.layout, (id) => {
        const pane = tab.pane(id);
        return {
          profile: pane?.profileId ?? null,
          // A shell that never reported its folder keeps the one it was restored into.
          cwd: pane?.cwd ?? pane?.restoreDirectory ?? null,
        };
      }),
    }));
    return {
      version: 1,
      activeTab: Math.max(0, this.tabs.findIndex((tab) => tab.id === this.activeId)),
      tabs,
    };
  }

  /**
   * Opens what a previous run saved, or keeps the fresh tab when there is nothing to restore. Either
   * way the workspace counts as loaded afterwards. Panes are new: only their layout, profile and folder come back.
   */
  restore(saved: Workspace | null): void {
    if (saved && saved.tabs.length > 0) {
      const restored = saved.tabs.map((entry: WorkspaceTab) => {
        const { layout, panes } = tree.fromNode(entry.root, newId);
        const made = panes.map((info) => new Pane(info.id, info.profile, info.cwd));
        const tab = new Tab(newId(), made[0]);
        tab.adopt(layout, made, entry.activePane);
        tab.name = entry.title;
        return tab;
      });
      this.tabs = restored;
      this.activeId = restored[Math.min(Math.max(saved.activeTab, 0), restored.length - 1)].id;
    }
    this.loaded = true;
  }

  get activeTab(): Tab {
    return this.tabs.find((tab) => tab.id === this.activeId) ?? this.tabs[0];
  }

  /** The pane that has the keyboard. */
  get activePane(): Pane {
    return this.activeTab.active;
  }

  /** Splits the active pane. `null` when the tab cannot hold another one. */
  splitActive(direction: SplitDirection, before = false): Pane | null {
    return this.activeTab.split(direction, before);
  }

  /** Closes the active pane; the last pane of a tab closes the tab. */
  closeActivePane(): void {
    const tab = this.activeTab;
    if (!tab.closePane(tab.activePaneId)) this.closeTab(tab.id);
  }

  /**
   * Opens a tab and switches to it. Without a profile it starts in the folder of the pane you were in,
   * like most terminals; choosing a profile starts in that profile's own folder. `null` at the limit
   * of tabs: past it the layout could no longer be saved.
   */
  newTab(profileId: string | null = null): Tab | null {
    if (this.tabs.length >= MAX_TABS) return null;
    const inherited = profileId === null && this.tabs.length > 0 ? this.activePane.cwd : null;
    const tab = new Tab(newId(), new Pane(newId(), profileId, inherited));
    this.tabs = [...this.tabs, tab];
    this.activeId = tab.id;
    return tab;
  }

  /** Closes a tab. Closing the last one opens a fresh tab, so the window is never empty. */
  closeTab(id: string): void {
    const index = this.tabs.findIndex((tab) => tab.id === id);
    if (index < 0) return;
    const remaining = this.tabs.filter((tab) => tab.id !== id);
    if (remaining.length === 0) {
      this.tabs = [];
      this.newTab();
      return;
    }
    this.tabs = remaining;
    if (this.activeId === id) {
      this.activeId = remaining[Math.min(index, remaining.length - 1)].id;
    }
  }

  activate(id: string): void {
    if (this.tabs.some((tab) => tab.id === id)) this.activeId = id;
  }

  /** Switches to the tab at a 0-based position; out of range does nothing. */
  activateAt(position: number): void {
    const tab = this.tabs[position];
    if (tab) this.activeId = tab.id;
  }

  /** Moves to the next (`1`) or previous (`-1`) tab, wrapping around. */
  cycle(step: 1 | -1): void {
    const index = this.tabs.findIndex((tab) => tab.id === this.activeId);
    const next = (index + step + this.tabs.length) % this.tabs.length;
    this.activeId = this.tabs[next].id;
  }

  /** Puts a tab at a new position, clamped to the ends. */
  moveTab(id: string, position: number): void {
    const from = this.tabs.findIndex((tab) => tab.id === id);
    if (from < 0) return;
    const to = Math.min(Math.max(position, 0), this.tabs.length - 1);
    if (to === from) return;
    const next = [...this.tabs];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    this.tabs = next;
  }

  /** Names a tab; an empty name goes back to following the shell. */
  renameTab(id: string, name: string): void {
    const tab = this.tabs.find((candidate) => candidate.id === id);
    if (!tab) return;
    const trimmed = name.trim().slice(0, 64);
    tab.name = trimmed || null;
  }
}

export const workspace = new WorkspaceStore();
