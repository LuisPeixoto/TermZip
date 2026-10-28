import { newId } from "../ids";
import { leaf, leaves, type Layout } from "../layout";
import { basename } from "../paths";
import { Pane } from "./pane.svelte";

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

  constructor() {
    this.newTab();
  }

  get activeTab(): Tab {
    return this.tabs.find((tab) => tab.id === this.activeId) ?? this.tabs[0];
  }

  /** The pane that has the keyboard. */
  get activePane(): Pane {
    return this.activeTab.active;
  }

  /**
   * Opens a tab and switches to it. Without a profile it starts in the folder of the pane you were in,
   * like most terminals; choosing a profile starts in that profile's own folder.
   */
  newTab(profileId: string | null = null): Tab {
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
