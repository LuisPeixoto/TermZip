<script lang="ts">
  import { app } from "../stores/app.svelte";
  import { workspace } from "../stores/workspace.svelte";
  import { pluginStore } from "../stores/plugins.svelte";

  const items = $derived(Object.entries(pluginStore.statuses));
</script>

<footer
  class="flex h-7 shrink-0 items-center gap-3 border-t border-line bg-window-panel px-3 text-[10.5px] text-muted"
>
  <span class="flex items-center gap-1.5">
    <span
      class="size-1.5 rounded-full {workspace.activePane.exited ? 'bg-warn' : 'bg-ok'}"
      title={workspace.activePane.exited ? "Shell exited" : "Shell running"}
    ></span>
    {#if workspace.activePane.exited}
      shell exited
    {:else if workspace.activePane.session?.integrated}
      shell integration
    {:else}
      plain terminal
    {/if}
  </span>

  <span class="text-line-strong">·</span>

  {#if workspace.activePane.session && !workspace.activePane.session.integrated}
    <span class="text-warn" title="This shell does not emit OSC 133 markers; blocks are unavailable">
      no block capture
    </span>
    <span class="text-line-strong">·</span>
  {/if}

  <span class="mono">
    {workspace.activePane.viewport.columns}×{workspace.activePane.viewport.rows}
  </span>

  {#each items as [plugin, text] (plugin)}
    <span class="text-line-strong">·</span>
    <span class="max-w-[16rem] truncate" title="From the plugin {pluginStore.name(plugin)}">{text}</span>
  {/each}

  <span class="flex-1"></span>

  <span class="hidden gap-3 text-faint md:flex" aria-label="Keyboard shortcuts">
    {#if app.shortcut("blocks")}<span><kbd class="mono">{app.shortcut("blocks")}</kbd> blocks</span>{/if}
    {#if app.shortcut("restart")}<span><kbd class="mono">{app.shortcut("restart")}</kbd> restart</span>{/if}
    {#if app.shortcut("preferences")}<span><kbd class="mono">{app.shortcut("preferences")}</kbd> preferences</span>{/if}
  </span>

  <span class="mono text-faint">
    TermZip {app.version}
  </span>
</footer>