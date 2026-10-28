import type { Workspace } from "../api";
import type { WorkspaceStore } from "./workspace.svelte";

export interface Persistence {
  /** Saves right away if something changed, for when the window is closing. */
  flush(): void;
  stop(): void;
}

/**
 * Saves the workspace a moment after it changes, so a burst of `cd`s or a divider drag costs one
 * write. Nothing is written for a layout that is already what was last saved.
 */
export function persistWorkspace(
  store: WorkspaceStore,
  save: (workspace: Workspace) => Promise<void>,
  delay = 500,
): Persistence {
  let last = JSON.stringify(store.snapshot());
  let timer: ReturnType<typeof setTimeout> | undefined;

  const write = (text: string) => {
    last = text;
    void save(JSON.parse(text) as Workspace).catch(() => {});
  };

  const stop = $effect.root(() => {
    $effect(() => {
      const text = JSON.stringify(store.snapshot());
      if (text === last) return;
      clearTimeout(timer);
      timer = setTimeout(() => write(text), delay);
    });
  });

  return {
    flush() {
      clearTimeout(timer);
      const text = JSON.stringify(store.snapshot());
      if (text !== last) write(text);
    },
    stop() {
      clearTimeout(timer);
      stop();
    },
  };
}
