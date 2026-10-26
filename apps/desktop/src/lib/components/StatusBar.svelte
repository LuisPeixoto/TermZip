<script lang="ts">
  import { app } from "../stores/app.svelte";
</script>

<footer
  class="flex h-7 shrink-0 items-center gap-3 border-t border-line bg-panel px-3 text-[10.5px] text-muted"
>
  <span class="flex items-center gap-1.5">
    <span
      class="size-1.5 rounded-full {app.exited ? 'bg-warn' : 'bg-ok'}"
      title={app.exited ? "Shell exited" : "Shell running"}
    ></span>
    {#if app.exited}
      shell exited
    {:else if app.session?.integrated}
      shell integration
    {:else}
      plain terminal
    {/if}
  </span>

  <span class="text-line-strong">·</span>

  {#if app.session && !app.session.integrated}
    <span class="text-warn" title="This shell does not emit OSC 133 markers; blocks are unavailable">
      no block capture
    </span>
    <span class="text-line-strong">·</span>
  {/if}

  <span class="mono">
    {app.viewport.columns}×{app.viewport.rows}
  </span>

  <span class="flex-1"></span>

  <span class="mono text-faint">
    TermZip {app.session ? "" : "starting…"}
  </span>
</footer>