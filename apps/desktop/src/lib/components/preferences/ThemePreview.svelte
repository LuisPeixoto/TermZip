<script lang="ts">
  import type { Theme } from "../../api";

  let { theme }: { theme: Theme } = $props();

  const colors = $derived(theme.colors);
</script>

<!-- A few lines of a terminal in the theme's own colors, so it can be judged before it is chosen. -->
<div
  class="mono pointer-events-none select-none overflow-hidden whitespace-nowrap px-2.5 py-2 text-[10px] leading-[1.5]"
  style="background:{colors.background};color:{colors.foreground}"
  aria-hidden="true"
>
  <div><span style="color:{colors.ansi[6]}">~/code</span> <span style="color:{colors.ansi[5]}">main</span></div>
  <div><span style="color:{colors.ansi[2]}">❯</span> ls</div>
  <div>
    <span style="color:{colors.ansi[4]};font-weight:600">src</span> <span style="color:{colors.ansi[2]}">run.sh</span> <span
      style="color:{colors.ansi[3]}">notes</span
    > README
  </div>
  <div><span style="color:{colors.ansi[1]}">✗</span> not found <span
      style="background:{colors.cursor};color:{colors.cursor_text ?? colors.background}"> </span></div>
</div>
