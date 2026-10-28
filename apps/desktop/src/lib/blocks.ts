import type { Block } from "./api";

/** Mirrors `MAX_STORED_BLOCKS` in the Rust tracker: the rail never holds more than the backend does. */
export const MAX_BLOCKS = 200;

/** Adds one finished block to the list, dropping the oldest ones past the cap. Returns a new array. */
export function appendBlock(list: readonly Block[], block: Block, max: number = MAX_BLOCKS): Block[] {
  // A block that is already there (a refresh raced the event) is replaced, never duplicated.
  const kept = list.filter((existing) => existing.index !== block.index);
  kept.push(block);
  return kept.length > max ? kept.slice(kept.length - max) : kept;
}
