<script lang="ts">
  import Icon from "./Icon.svelte";
  import { app } from "../stores/app.svelte";
</script>

<header
  class="flex h-11 shrink-0 items-center gap-2 border-b border-line bg-panel pr-3
    {app.platform === 'macos' ? 'pl-[76px]' : 'pl-3'}"
  data-tauri-drag-region
>
  <div class="flex items-center gap-2" data-tauri-drag-region>
    <span
      class="grid size-6 place-items-center rounded-md bg-panel-2 text-accent ring-1 ring-line"
      data-tauri-drag-region
    >
      <Icon name="terminal" size={13} stroke={2.2} />
    </span>
    <span class="text-[12.5px] font-semibold tracking-tight" data-tauri-drag-region>TermZip</span>
  </div>

  <span class="ml-1 max-w-[280px] truncate rounded-full border border-line bg-panel-2 px-2 py-0.5 text-[10.5px] text-muted">
    {app.title || app.session?.shell || "starting…"}
  </span>

  {#if app.exited}
    <button
      type="button"
      class="flex h-7 items-center gap-1.5 rounded-md border border-warn/40 bg-warn/10 px-2 text-[11px] text-warn transition-colors hover:bg-warn/20"
      onclick={() => window.dispatchEvent(new CustomEvent("termzip:restart"))}
      title="Restart the shell (⌘R)"
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
    title="Toggle command blocks (⌘B)"
  >
    <Icon name="layers" size={13} />
    Blocks
  </button>

  <button
    type="button"
    class="grid size-7 place-items-center rounded-md border border-line bg-panel text-muted transition-colors hover:bg-hover hover:text-fg"
    onclick={() => app.toggleTheme()}
    title={app.settings.theme === "dark" ? "Switch to light theme" : "Switch to dark theme"}
  >
    <Icon name={app.settings.theme === "dark" ? "sun" : "moon"} size={14} />
  </button>

  <button
    type="button"
    class="grid size-7 place-items-center rounded-md border border-line bg-panel text-muted transition-colors hover:bg-hover hover:text-fg"
    onclick={() => app.toggleSettings()}
    title="Preferences (⌘,)"
  >
    <Icon name="settings" size={14} />
  </button>
</header>