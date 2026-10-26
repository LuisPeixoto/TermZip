<script lang="ts">
  import Icon from "./Icon.svelte";
  import { app } from "../stores/app.svelte";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { api, type Block } from "../api";

  let copiedIndex: number | null = $state(null);

  function basename(path: string | null): string | null {
    if (!path) return null;
    const trimmed = path.replace(/\/+$/, "");
    const last = trimmed.split(/[\\/]/).pop();
    return last || path;
  }

  function formatDuration(ms: number | null): string | null {
    if (ms == null) return null;
    if (ms < 1000) return `${ms} ms`;
    const seconds = ms / 1000;
    if (seconds < 60) return `${seconds.toFixed(seconds < 10 ? 2 : 1)} s`;
    const minutes = Math.floor(seconds / 60);
    return `${minutes}m ${Math.round(seconds % 60)}s`;
  }

  function statusClass(block: Block): string {
    const ok = block.exitCode === 0;
    return `${ok ? "bg-ok/15 text-ok" : "bg-err/15 text-err"}`;
  }

  function newestIndex(): number | null {
    return app.blocks.length > 0 ? app.blocks[app.blocks.length - 1].index : null;
  }

  async function copyCommand(index: number, command: string) {
    try {
      await writeText(command);
      copiedIndex = index;
      window.setTimeout(() => (copiedIndex = null), 1200);
    } catch {
      app.toast("warn", "Clipboard is unavailable");
    }
  }

  async function rerun(command: string) {
    window.dispatchEvent(
      new CustomEvent("termzip:composer", { detail: { command, execute: true } }),
    );
  }

  async function refresh() {
    const session = app.session;
    if (!session) return;
    try {
      app.setBlocks(await api.listBlocks(session.id));
    } catch {
      /* the session may be gone */
    }
  }
</script>

<aside class="flex w-[300px] shrink-0 flex-col border-l border-line bg-panel">
  <div class="flex h-9 items-center justify-between border-b border-line px-3">
    <span class="text-[11px] font-semibold uppercase tracking-wide text-muted">Command blocks</span>
    <button
      type="button"
      class="grid size-6 place-items-center rounded text-faint transition-colors hover:bg-hover hover:text-fg"
      onclick={() => void refresh()}
      title="Refresh"
    >
      <Icon name="refresh" size={13} />
    </button>
  </div>

  {#if app.blocks.length === 0}
    <div class="px-4 py-6 text-[11.5px] leading-relaxed text-faint">
      Commands you run appear here with their exit code and duration. TermZip reads
      <span class="mono">OSC&nbsp;133</span> markers from your shell.
    </div>
  {:else}
    <div class="min-h-0 flex-1 overflow-y-auto p-2">
      {#each [...app.blocks].reverse() as block (block.index)}
        <article
          class="mb-1.5 rounded-lg border border-line bg-panel-2 p-2.5
            {block.index === newestIndex() ? 'block-new' : ''}"
        >
          <div class="flex items-center justify-between gap-2">
            <span class="text-[10px] font-medium text-faint">#{block.index}</span>
            <div class="flex items-center gap-1.5">
              {#if block.exitCode != null}
                <span
                  class="mono rounded px-1.5 py-0.5 text-[9.5px] font-semibold {statusClass(block)}"
                  title="Exit code"
                >
                  {block.exitCode === 0 ? "ok" : `exit ${block.exitCode}`}
                </span>
              {/if}
              {#if formatDuration(block.durationMs) != null}
                <span class="mono flex items-center gap-1 text-[9.5px] text-faint" title="Duration">
                  <Icon name="clock" size={10} stroke={2} />
                  {formatDuration(block.durationMs)}
                </span>
              {/if}
            </div>
          </div>

          {#if block.command}
            <p class="mono selectable mt-1.5 line-clamp-3 break-all text-[11.5px] text-fg">
              {block.command}
            </p>
          {:else}
            <p class="mono mt-1.5 text-[11.5px] italic text-faint">command not captured</p>
          {/if}

          <div class="mt-2 flex items-center gap-1">
            {#if block.command}
              <button
                type="button"
                class="flex items-center gap-1 rounded border border-line bg-panel px-1.5 py-0.5 text-[10px] text-muted transition-colors hover:bg-hover hover:text-fg"
                onclick={() => block.command && copyCommand(block.index, block.command)}
              >
                <Icon name={copiedIndex === block.index ? "check" : "copy"} size={11} />
                {copiedIndex === block.index ? "Copied" : "Copy"}
              </button>
              <button
                type="button"
                class="flex items-center gap-1 rounded border border-line bg-panel px-1.5 py-0.5 text-[10px] text-muted transition-colors hover:bg-hover hover:text-fg"
                onclick={() => block.command && rerun(block.command)}
              >
                <Icon name="zap" size={11} />
                Run
              </button>
            {/if}
            <span class="flex-1"></span>
            {#if basename(block.cwd)}
              <span class="mono flex items-center gap-1 text-[9.5px] text-faint" title={block.cwd ?? ""}>
                <Icon name="folder" size={10} stroke={2} />
                {basename(block.cwd)}
              </span>
            {/if}
          </div>
        </article>
      {/each}
    </div>
  {/if}
</aside>