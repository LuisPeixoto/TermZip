<script lang="ts">
  import Icon from "./Icon.svelte";
  import BlockCard from "./BlockCard.svelte";
  import VirtualList from "./VirtualList.svelte";
  import { app } from "../stores/app.svelte";
  import { workspace } from "../stores/workspace.svelte";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { api } from "../api";
  import { filterBlocks, type StatusFilter } from "../blocks";

  const pane = $derived(workspace.activePane);
  const visible = $derived(filterBlocks(pane.blocks, pane.blockQuery, pane.blockStatus));
  // Newest first.
  const ordered = $derived([...visible].reverse());
  const filtering = $derived(pane.blockQuery.trim() !== "" || pane.blockStatus !== "all");
  const STATUSES: Array<[StatusFilter, string]> = [
    ["all", "All"],
    ["ok", "OK"],
    ["failed", "Failed"],
  ];
  let copied = $state<string | null>(null);
  let expanded = $state<Set<number>>(new Set());

  // Block numbers start again with every shell, so what was expanded belongs to the old one.
  $effect(() => {
    pane.id;
    pane.session?.id;
    expanded = new Set();
  });

  function toggle(index: number) {
    const next = new Set(expanded);
    if (next.has(index)) next.delete(index);
    else next.add(index);
    expanded = next;
  }

  function isExpanded(index: number): boolean {
    return expanded.has(index);
  }

  function clearFilter() {
    pane.blockQuery = "";
    pane.blockStatus = "all";
  }

  async function copy(key: string, text: string) {
    try {
      await writeText(text);
      copied = key;
      window.setTimeout(() => (copied = null), 1200);
    } catch {
      app.toast("warn", "Clipboard is unavailable");
    }
  }

  async function rerun(command: string) {
    if (await pane.write(`${command}\r`)) {
      window.dispatchEvent(new CustomEvent("termzip:focus-terminal"));
    }
  }

  async function refresh() {
    const session = pane.session;
    if (!session) return;
    try {
      pane.setBlocks(await api.listBlocks(session.id));
    } catch {
      /* the session may be gone */
    }
  }
</script>

<aside class="flex w-[clamp(280px,30vw,360px)] shrink-0 flex-col border-l border-line bg-window-panel">
  <div class="flex h-9 items-center justify-between border-b border-line px-3">
    <span class="flex items-center gap-1.5 text-[11px] font-semibold uppercase tracking-wide text-muted">
      <Icon name="layers" size={12} />
      Command blocks
    </span>
    <button
      type="button"
      class="grid size-6 place-items-center rounded text-faint transition-colors hover:bg-hover hover:text-fg"
      onclick={() => void refresh()}
      title="Refresh blocks"
      aria-label="Refresh blocks"
    >
      <Icon name="refresh" size={13} />
    </button>
  </div>

  {#if pane.blocks.length === 0}
    <div class="px-4 py-6 text-[11.5px] leading-relaxed text-faint">
      Run a command to create your first block. TermZip reads
      <span class="mono">OSC&nbsp;133</span> markers from zsh, bash, fish, and PowerShell.
    </div>
  {:else}
    <div class="space-y-1.5 border-b border-line p-2">
      <input
        type="search"
        spellcheck="false"
        placeholder="Filter blocks"
        aria-label="Filter blocks by text"
        bind:value={pane.blockQuery}
        class="h-7 w-full rounded-md border border-line bg-panel-2 px-2 text-[11.5px] text-fg outline-none placeholder:text-faint focus:border-accent"
        onkeydown={(event) => {
          // Escape empties a filter first; with nothing to clear it leaves the box as usual.
          if (event.key === "Escape" && pane.blockQuery) {
            pane.blockQuery = "";
            event.stopPropagation();
          }
        }}
      />
      <div class="flex items-center gap-1" role="group" aria-label="Filter blocks by status">
        {#each STATUSES as [value, label] (value)}
          <button
            type="button"
            class="rounded px-2 py-0.5 text-[10.5px] font-medium transition-colors
              {pane.blockStatus === value ? 'bg-accent-soft text-accent' : 'text-muted hover:bg-hover hover:text-fg'}"
            aria-pressed={pane.blockStatus === value}
            onclick={() => (pane.blockStatus = value)}
          >
            {label}
          </button>
        {/each}
        <span class="ml-auto text-[10.5px] text-faint" aria-live="polite">
          {filtering ? `${visible.length} of ${pane.blocks.length}` : `${pane.blocks.length}`}
        </span>
      </div>
    </div>

    {#if visible.length === 0}
      <div class="px-4 py-6 text-[11.5px] leading-relaxed text-faint">
        No block matches.
        <button type="button" class="text-accent underline-offset-2 hover:underline" onclick={clearFilter}>
          Clear the filter
        </button>
      </div>
    {/if}

    <VirtualList items={ordered} keyOf={(entry) => entry.index} class="min-h-0 flex-1 overflow-y-auto p-2">
      {#snippet row(block)}
        <BlockCard
          {block}
          expanded={isExpanded(block.index)}
          {copied}
          ontoggle={() => toggle(block.index)}
          oncopy={(key, text) => void copy(key, text)}
          onrerun={(command) => void rerun(command)}
        />
      {/snippet}
    </VirtualList>
  {/if}
</aside>