<script lang="ts">
  import type { CursorStyle } from "../../api";
  import { fontStack } from "../../terminal/font";

  let {
    fontSize,
    fontFamily,
    lineHeight,
    letterSpacing,
    cursorStyle,
    cursorBlink,
  }: {
    fontSize: number;
    fontFamily: string | null;
    lineHeight: number;
    letterSpacing: number;
    cursorStyle: CursorStyle;
    cursorBlink: boolean;
  } = $props();
</script>

<!-- What the terminal will look like with these settings: its own font, spacing and cursor. -->
<div
  class="pointer-events-none select-none overflow-hidden rounded-lg border border-line bg-bg px-3 py-2 text-fg"
  style="font-family:{fontStack(fontFamily)};font-size:{fontSize}px;line-height:{lineHeight};letter-spacing:{letterSpacing}px"
  aria-hidden="true"
>
  <div><span class="text-ok">❯</span> git status</div>
  <div class="text-muted">On branch main, nothing to commit</div>
  <div>
    <span class="text-ok">❯</span> ls<span
      class="cursor {cursorStyle}"
      class:blink={cursorBlink}
    ></span>
  </div>
</div>

<style>
  .cursor {
    display: inline-block;
    width: 0.6em;
    height: 1em;
    margin-left: 0.4em;
    vertical-align: text-bottom;
    background: var(--accent);
  }
  .cursor.bar {
    width: 2px;
  }
  .cursor.underline {
    height: 2px;
    vertical-align: baseline;
  }
  @media (prefers-reduced-motion: no-preference) {
    .cursor.blink {
      animation: cursor-blink 1.1s steps(1) infinite;
    }
  }
  @keyframes cursor-blink {
    50% {
      opacity: 0;
    }
  }
</style>
