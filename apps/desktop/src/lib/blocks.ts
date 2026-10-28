import type { Block } from "./api";
import { stripAnsi } from "./ansi";

/** Mirrors `MAX_STORED_BLOCKS` in the Rust tracker: the rail never holds more than the backend does. */
export const MAX_BLOCKS = 200;

/** Adds one finished block to the list, dropping the oldest ones past the cap. Returns a new array. */
export function appendBlock(list: readonly Block[], block: Block, max: number = MAX_BLOCKS): Block[] {
  // A block that is already there (a refresh raced the event) is replaced, never duplicated.
  const kept = list.filter((existing) => existing.index !== block.index);
  kept.push(block);
  return kept.length > max ? kept.slice(kept.length - max) : kept;
}

export type StatusFilter = "all" | "ok" | "failed";

/** A block that ran to the end with exit code 0 is `ok`; any other code is `failed`; no code is `unknown`. */
export function blockStatus(block: Block): "ok" | "failed" | "unknown" {
  if (block.exitCode === null) return "unknown";
  return block.exitCode === 0 ? "ok" : "failed";
}

// A block never changes once it is finished, so what a person sees in it is worked out once.
const readable = new WeakMap<Block, string>();

function readableText(block: Block): string {
  let text = readable.get(block);
  if (text === undefined) {
    text = `${block.command ?? ""}\n${stripAnsi(block.output ?? "")}`.toLowerCase();
    readable.set(block, text);
  }
  return text;
}

/**
 * The blocks that match what the user is looking for: some text (anywhere in the command or in what it
 * printed, ignoring case and color codes) and a status. Unfinished or code-less blocks only show under "all".
 */
export function filterBlocks(blocks: readonly Block[], query: string, status: StatusFilter): Block[] {
  const needle = query.trim().toLowerCase();
  if (!needle && status === "all") return [...blocks];
  return blocks.filter((block) => {
    if (status !== "all" && blockStatus(block) !== status) return false;
    return !needle || readableText(block).includes(needle);
  });
}
