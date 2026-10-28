<script lang="ts">
  import TerminalPane from "./TerminalPane.svelte";
  import { dividers, ratioAt, rects, type Divider } from "../layout";
  import type { Tab } from "../stores/workspace.svelte";

  let { tab }: { tab: Tab } = $props();

  let host: HTMLDivElement | undefined = $state();
  let dragging = $state<string | null>(null);

  const placed = $derived(rects(tab.layout));
  const panes = $derived(tab.paneList);
  const lines = $derived(dividers(tab.layout));
  const several = $derived(panes.length > 1);

  const percent = (fraction: number) => `${(fraction * 100).toFixed(4)}%`;

  // Panes are laid out from the layout's rectangles instead of nested boxes, so the terminal of a
  // pane keeps its place in the DOM (and its shell) when another pane is split off or closed.
  function lineStyle(line: Divider): string {
    const { area, ratio, direction } = line;
    return direction === "row"
      ? `left:calc(${percent(area.x + area.w * ratio)} - 3px);top:${percent(area.y)};height:${percent(area.h)};width:6px`
      : `top:calc(${percent(area.y + area.h * ratio)} - 3px);left:${percent(area.x)};width:${percent(area.w)};height:6px`;
  }

  function startDrag(event: PointerEvent, line: Divider) {
    if (event.button !== 0 || !host) return;
    event.preventDefault();
    dragging = line.id;
    const move = (pointer: PointerEvent) => {
      const box = host!.getBoundingClientRect();
      const x = (pointer.clientX - box.left) / box.width;
      const y = (pointer.clientY - box.top) / box.height;
      tab.setRatio(line.id, ratioAt(line, x, y));
    };
    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", stop);
      window.removeEventListener("pointercancel", stop);
      dragging = null;
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stop);
    window.addEventListener("pointercancel", stop);
  }
</script>

<div bind:this={host} class="relative min-h-0 min-w-0 flex-1">
  {#each panes as pane (pane.id)}
    {@const box = placed.get(pane.id)}
    {#if box}
      <div
        class="absolute flex min-h-0 min-w-0 flex-col"
        style="left:{percent(box.x)};top:{percent(box.y)};width:{percent(box.w)};height:{percent(box.h)}"
        role="group"
        aria-label="Terminal pane"
        onfocusin={() => tab.focus(pane.id)}
        onpointerdowncapture={() => tab.focus(pane.id)}
      >
        <TerminalPane {pane} />
        <!-- Drawn on top: the terminal paints its own background over an inset ring on the wrapper. -->
        {#if several && pane.id === tab.activePaneId}
          <span class="pointer-events-none absolute inset-0 ring-1 ring-inset ring-accent/60"></span>
        {/if}
      </div>
    {/if}
  {/each}

  {#each lines as line (line.id)}
    <div
      class="group absolute z-10 {line.direction === 'row' ? 'cursor-col-resize' : 'cursor-row-resize'}"
      style={lineStyle(line)}
      role="separator"
      aria-orientation={line.direction === "row" ? "vertical" : "horizontal"}
      onpointerdown={(event) => startDrag(event, line)}
    >
      <span
        class="absolute transition-colors
          {line.direction === 'row' ? 'inset-y-0 left-[2px] w-[2px]' : 'inset-x-0 top-[2px] h-[2px]'}
          {dragging === line.id ? 'bg-accent' : 'bg-line group-hover:bg-accent/70'}"
      ></span>
    </div>
  {/each}
</div>
