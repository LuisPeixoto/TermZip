import { api, type GitStatus } from "../api";

/** How long after the last trigger git is asked; a burst of `cd`s or commands costs one run. */
const DELAY_MS = 250;

/**
 * The git state of the folder of the active terminal. It is read after the folder changes, after a command
 * finishes and when the window comes back to the front, never on a timer, and an answer for a folder the
 * terminal has already left is dropped.
 */
class GitStore {
  status = $state.raw<GitStatus | null>(null);
  #timer: ReturnType<typeof setTimeout> | undefined;
  #generation = 0;
  #folder: string | null = null;

  /** Follow `folder` (`null` for none: no terminal, a remote one, or the setting is off). */
  watch(folder: string | null): void {
    this.#folder = folder;
    clearTimeout(this.#timer);
    const generation = ++this.#generation;
    if (folder === null) {
      this.status = null;
      return;
    }
    this.#timer = setTimeout(() => {
      api
        .gitStatus(folder)
        .then((status) => {
          if (generation === this.#generation) this.status = status;
        })
        .catch(() => {
          if (generation === this.#generation) this.status = null;
        });
    }, DELAY_MS);
  }

  /** Ask again for the same folder, for something that happened outside the app. */
  refresh(): void {
    if (this.#folder !== null) this.watch(this.#folder);
  }
}

export const git = new GitStore();
