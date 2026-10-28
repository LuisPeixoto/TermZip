<script lang="ts">
  import Icon from "./Icon.svelte";
  import AnsiOutput from "./AnsiOutput.svelte";
  import { app } from "../stores/app.svelte";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { api, type Block } from "../api";

  let copied = $state<string | null>(null);
  let expanded = $state<Set<number>>(new Set());

  // Block numbers start again with every shell, so what was expanded belongs to the old one.
  $effect(() => {
    app.session?.id;
    expanded = new Set();
  });

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

  function toggle(index: number) {
    const next = new Set(expanded);
    if (next.has(index)) next.delete(index);
    else next.add(index);
    expanded = next;
  }

  function isExpanded(index: number): boolean {
    return expanded.has(index);
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
    if (await app.writeToShell(`${command}\r`)) {
      window.dispatchEvent(new CustomEvent("termzip:focus-terminal"));
    }
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

  function statusClass(block: Block): string {
    return block.exitCode === 0 ? "bg-ok/15 text-ok" : "bg-err/15 text-err";
  }
</script>

<aside class="flex w-[clamp(280px,30vw,360px)] shrink-0 flex-col border-l border-line bg-panel">
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

  {#if app.blocks.length === 0}
    <div class="px-4 py-6 text-[11.5px] leading-relaxed text-faint">
      Run a command to create your first block. TermZip reads
      <span class="mono">OSC&nbsp;133</span> markers from zsh, bash, fish, and PowerShell.
    </div>
  {:else}
    <div class="min-h-0 flex-1 space-y-1.5 overflow-y-auto p-2">
      {#each [...app.blocks].reverse() as block (block.index)}
        <article
          class="rounded-lg border border-line bg-panel-2 transition-colors hover:border-line-strong"
        >
          <div class="p-2.5">
            <div class="flex items-center justify-between gap-2">
              <button
                type="button"
                class="flex min-w-0 items-center gap-1.5 text-left"
                onclick={() => toggle(block.index)}
                aria-expanded={isExpanded(block.index)}
              >
                <Icon
                  name="chevronRight"
                  size={11}
                  class="shrink-0 text-faint transition-transform {isExpanded(block.index) ? 'rotate-90' : ''}"
                />
                <span class="mono truncate text-[11.5px] text-fg">
                  {block.command ?? "command not captured"}
                </span>
              </button>
              <div class="flex shrink-0 items-center gap-1.5">
                {#if block.exitCode != null}
                  <span
                    class="mono rounded px-1.5 py-0.5 text-[9.5px] font-semibold {statusClass(block)}"
                    title="Exit code"
                  >
                    {block.exitCode === 0 ? "ok" : `exit ${block.exitCode}`}
                  </span>
                {/if}
                {#if formatDuration(block.durationMs) != null}
                  <span class="mono text-[9.5px] text-faint" title="Duration">
                    {formatDuration(block.durationMs)}
                  </span>
                {/if}
              </div>
            </div>

            {#if block.output}
              <div
                class="mt-2 rounded-md border border-line bg-bg/60 p-2"
                class:max-h-44={!isExpanded(block.index)}
                class:overflow-hidden={!isExpanded(block.index)}
              >
                <AnsiOutput
                  output={block.output}
                  maxHeight={isExpanded(block.index) ? 420 : 150}
                />
              </div>
              {#if block.truncated}
                <p class="mt-1 text-[9.5px] text-warn">
                  Output truncated in the block; open the terminal for the full text.
                </p>
              {/if}
            {:else}
              <p class="mt-2 text-[10px] italic text-faint">no output captured</p>
            {/if}

            <div class="mt-2 flex items-center gap-1">
              {#if block.command}
                <button
                  type="button"
                  class="flex items-center gap-1 rounded border border-line bg-panel px-1.5 py-0.5 text-[10px] text-muted transition-colors hover:bg-hover hover:text-fg"
                  onclick={() => block.command && copy(`cmd-${block.index}`, block.command)}
                >
                  <Icon name={copied === `cmd-${block.index}` ? "check" : "copy"} size={11} />
                  Copy
                </button>
                <button
                  type="button"
                  class="flex items-center gap-1 rounded border border-line bg-panel px-1.5 py-0.5 text-[10px] text-muted transition-colors hover:bg-hover hover:text-fg"
                  onclick={() => block.command && void rerun(block.command)}
                >
                  <Icon name="zap" size={11} />
                  Run
                </button>
              {/if}
              {#if block.output}
                <button
                  type="button"
                  class="flex items-center gap-1 rounded border border-line bg-panel px-1.5 py-0.5 text-[10px] text-muted transition-colors hover:bg-hover hover:text-fg"
                  onclick={() => block.output && copy(`out-${block.index}`, block.output)}
                >
                  <Icon name={copied === `out-${block.index}` ? "check" : "copy"} size={11} />
                  Output
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
          </div>
        </article>
      {/each}
    </div>
  {/if}
</aside>