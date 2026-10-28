<script lang="ts">
  import { ansiToSpans, DEFAULT_PALETTE } from "../ansi";
  import { ansiPalette } from "../theme";
  import { app } from "../stores/app.svelte";

  let { output, maxHeight = 260 }: { output: string; maxHeight?: number } = $props();

  // Follows the theme in use; the built-in palette only covers the moment before themes have loaded.
  let palette = $derived(app.theme ? ansiPalette(app.theme) : DEFAULT_PALETTE);
  let spans = $derived(ansiToSpans(output ?? "", palette));
</script>

<pre
  class="mono selectable m-0 overflow-auto whitespace-pre-wrap break-words text-[11.5px] leading-[1.55] text-fg"
  style="max-height:{maxHeight}px"
>{#each spans as span, index (index)}<span
      style:color={span.style.inverse ? span.style.bg : span.style.fg}
      style:background-color={span.style.inverse ? span.style.fg : span.style.bg}
      style:font-weight={span.style.bold ? 600 : undefined}
      style:opacity={span.style.dim ? 0.65 : undefined}
      style:font-style={span.style.italic ? "italic" : undefined}
      style:text-decoration={span.style.underline ? "underline" : undefined}
    >{span.text}</span>{/each}</pre>