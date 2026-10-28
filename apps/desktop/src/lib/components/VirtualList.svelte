<script lang="ts" generics="T">
  import type { Snippet } from "svelte";
  import {
    anchorAt,
    offsetsFor,
    scrollTopFor,
    visibleRange,
    type Anchor,
    type Key,
  } from "../virtual";

  let {
    items,
    keyOf,
    row,
    class: className = "",
  }: {
    items: readonly T[];
    keyOf: (item: T) => Key;
    row: Snippet<[T]>;
    class?: string;
  } = $props();

  let scroller: HTMLDivElement | undefined = $state();
  let scrollTop = $state(0);
  let viewport = $state(0);
  // Heights are measured once an item is on screen and kept, so scrolling back uses the real ones.
  let heights = $state.raw(new Map<Key, number>());
  let anchor: Anchor | null = null;
  let flushing = false;
  const pending = new Map<Key, number>();

  const keys = $derived(items.map(keyOf));
  const offsets = $derived(offsetsFor(keys, heights));
  const range = $derived(visibleRange(offsets, scrollTop, viewport));
  const shown = $derived(items.slice(range.start, range.end));
  const before = $derived(offsets[range.start] ?? 0);
  const after = $derived((offsets[offsets.length - 1] ?? 0) - (offsets[range.end] ?? 0));

  function onScroll() {
    if (!scroller) return;
    scrollTop = scroller.scrollTop;
    anchor = scrollTop > 8 ? anchorAt(keys, offsets, scrollTop) : null;
  }

  // Measurements arrive in bursts (every card that mounts); apply them together, once per frame.
  function flush() {
    flushing = false;
    const next = new Map(heights);
    for (const [key, height] of pending) next.set(key, height);
    pending.clear();
    // Items that left the list (old blocks, a narrower filter) must not pile up for the life of the session.
    if (next.size > keys.length * 2 + 20) {
      const present = new Set(keys);
      for (const key of next.keys()) if (!present.has(key)) next.delete(key);
    }
    heights = next;
  }

  function measure(node: HTMLElement, key: Key) {
    let current = key;
    const observer = new ResizeObserver(() => {
      const height = node.offsetHeight;
      if (heights.get(current) === height && !pending.has(current)) return;
      pending.set(current, height);
      if (!flushing) {
        flushing = true;
        requestAnimationFrame(flush);
      }
    });
    observer.observe(node);
    return {
      update(next: Key) {
        current = next;
      },
      destroy() {
        observer.disconnect();
      },
    };
  }

  // When something above the reader's place changes size (a new block on top, a card measured or
  // expanded), keep the item they are looking at where it was instead of letting the list jump.
  $effect(() => {
    const target = anchor ? scrollTopFor(keys, offsets, anchor) : null;
    if (scroller && target !== null && Math.abs(target - scroller.scrollTop) > 1) {
      scroller.scrollTop = target;
    }
  });
</script>

<div bind:this={scroller} bind:clientHeight={viewport} class={className} onscroll={onScroll}>
  <div style="padding-top:{before}px;padding-bottom:{after}px">
    {#each shown as item (keyOf(item))}
      <div use:measure={keyOf(item)} class="pb-1.5">
        {@render row(item)}
      </div>
    {/each}
  </div>
</div>
