<script lang="ts">
  import Icon from "./Icon.svelte";
  import TabBar from "./TabBar.svelte";
  import { app } from "../stores/app.svelte";
  import { workspace } from "../stores/workspace.svelte";
  import { shortcutLabel } from "../shortcuts";
</script>

<header
  class="flex h-11 shrink-0 items-center gap-2 border-b border-line bg-panel pr-3
    {app.platform === 'macos' ? 'pl-[76px]' : 'pl-3'}"
  data-tauri-drag-region
>
  <span
    class="grid size-6 shrink-0 place-items-center rounded-md bg-panel-2 text-accent ring-1 ring-line"
    data-tauri-drag-region
  >
    <Icon name="terminal" size={13} stroke={2.2} />
  </span>

  <TabBar />

  {#if workspace.activePane.exited}
    <button
      type="button"
      class="flex h-7 shrink-0 items-center gap-1.5 rounded-md border border-warn/40 bg-warn/10 px-2 text-[11px] text-warn transition-colors hover:bg-warn/20"
      onclick={() => window.dispatchEvent(new CustomEvent("termzip:restart"))}
      title="Restart the shell ({shortcutLabel('restart', app.platform)})"
    >
      Restart
    </button>
  {/if}

  <div class="flex-1" data-tauri-drag-region></div>

  <button
    type="button"
    class="flex h-7 items-center gap-1.5 rounded-md border px-2 text-[11px] transition-colors hover:bg-hover hover:text-fg
      {app.blocksOpen ? 'border-accent text-accent' : 'border-line text-muted'}"
    onclick={() => app.toggleBlocks()}
    title="Toggle command blocks ({shortcutLabel('blocks', app.platform)})"
  >
    <Icon name="layers" size={13} />
    Blocks
  </button>

  <button
    type="button"
    class="grid size-7 place-items-center rounded-md border border-line bg-panel text-muted transition-colors hover:bg-hover hover:text-fg"
    onclick={() => app.toggleTheme()}
    title={app.settings.theme === "dark" ? "Switch to light theme" : "Switch to dark theme"}
    aria-label={app.settings.theme === "dark" ? "Switch to light theme" : "Switch to dark theme"}
  >
    <Icon name={app.settings.theme === "dark" ? "sun" : "moon"} size={14} />
  </button>

  <button
    type="button"
    class="grid size-7 place-items-center rounded-md border border-line bg-panel text-muted transition-colors hover:bg-hover hover:text-fg"
    onclick={() => app.toggleSettings()}
    title="Preferences ({shortcutLabel('preferences', app.platform)})"
    aria-label="Preferences"
  >
    <Icon name="settings" size={14} />
  </button>
</header>