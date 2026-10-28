import { newId } from "../ids";
import { Pane } from "./pane.svelte";

/** The panes on screen. For now a single one; tabs and splits build on this. */
class WorkspaceStore {
  private pane = new Pane(newId());

  /** The pane that has the keyboard. */
  get activePane(): Pane {
    return this.pane;
  }
}

export const workspace = new WorkspaceStore();
