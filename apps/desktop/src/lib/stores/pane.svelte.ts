import { api, type Block, type SessionInfo } from "../api";
import { appendBlock, type StatusFilter } from "../blocks";
import { NO_RESULTS, type SearchResults } from "../terminal/search";
import { toast } from "./toasts.svelte";

/** What the terminal component lets the rest of the app do to its pane. */
export interface PaneController {
  restart(): void;
  focus(): void;
}

/** One shell and everything the interface shows about it. */
export class Pane {
  readonly id: string;
  /** The profile this pane starts with; `null` is the default profile. */
  profileId: string | null;
  /** Where the shell was when the layout was saved. Used for the first start only. */
  restoreDirectory: string | null;

  session = $state.raw<SessionInfo | null>(null);
  exited = $state(false);
  exitCode = $state<number | null>(null);
  title = $state<string | null>(null);
  cwd = $state<string | null>(null);
  viewport = $state({ rows: 0, columns: 0 });
  // Replaced as a whole on every change, so there is nothing to gain from a deep proxy.
  blocks = $state.raw<Block[]>([]);

  /** What the blocks rail is narrowed to for this pane. */
  blockQuery = $state("");
  blockStatus = $state<StatusFilter>("all");

  /** The find box of this pane, and what its last search found. */
  searchOpen = $state(false);
  searchResults = $state.raw<SearchResults>(NO_RESULTS);
  /** Goes up on every request to find, so an open box takes the keyboard again. */
  searchFocus = $state(0);

  /** Set by the terminal component while it is mounted. */
  controller: PaneController | null = null;

  constructor(id: string, profileId: string | null = null, restoreDirectory: string | null = null) {
    this.id = id;
    this.profileId = profileId;
    this.restoreDirectory = restoreDirectory;
  }

  /** Clears what belonged to the previous shell. Called before a new one opens, because its events can arrive first. */
  begin(): void {
    this.session = null;
    this.exited = false;
    this.exitCode = null;
    this.title = null;
    this.cwd = null;
    this.blocks = [];
  }

  openSearch(): void {
    this.searchOpen = true;
    this.searchFocus += 1;
  }

  closeSearch(): void {
    this.searchOpen = false;
    this.searchResults = NO_RESULTS;
  }

  setSession(info: SessionInfo): void {
    this.session = info;
  }

  markExited(code: number | null): void {
    this.exited = true;
    this.exitCode = code;
  }

  addBlock(block: Block): void {
    this.blocks = appendBlock(this.blocks, block);
  }

  setBlocks(list: Block[]): void {
    this.blocks = list;
  }

  /** Types text into the running shell, as if it had been entered in the terminal. */
  async write(text: string): Promise<boolean> {
    if (!this.session) {
      toast("warn", "The shell is not running yet");
      return false;
    }
    if (this.exited) {
      toast("err", "The shell has exited. Use Restart in the toolbar");
      return false;
    }
    try {
      await api.writeSession(this.session.id, new TextEncoder().encode(text));
      return true;
    } catch (error) {
      toast("err", `Could not send the command: ${error}`);
      return false;
    }
  }
}
