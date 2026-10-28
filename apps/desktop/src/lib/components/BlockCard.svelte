<script lang="ts">
  import Icon from "./Icon.svelte";
  import AnsiOutput from "./AnsiOutput.svelte";
  import type { Block } from "../api";
  import { basename } from "../paths";

  let {
    block,
    expanded,
    copied,
    ontoggle,
    oncopy,
    onrerun,
  }: {
    block: Block;
    expanded: boolean;
    /** The key of the button that was just pressed, to show a check mark for a moment. */
    copied: string | null;
    ontoggle: () => void;
    oncopy: (key: string, text: string) => void;
    onrerun: (command: string) => void;
  } = $props();

  function formatDuration(ms: number | null): string | null {
    if (ms == null) return null;
    if (ms < 1000) return `${ms} ms`;
    const seconds = ms / 1000;
    if (seconds < 60) return `${seconds.toFixed(seconds < 10 ? 2 : 1)} s`;
    const minutes = Math.floor(seconds / 60);
    return `${minutes}m ${Math.round(seconds % 60)}s`;
  }

  function statusClass(entry: Block): string {
    return entry.exitCode === 0 ? "bg-ok/15 text-ok" : "bg-err/15 text-err";
  }
</script>

<article
  class="rounded-lg border border-line bg-panel-2 transition-colors hover:border-line-strong"
>
  <div class="p-2.5">
    <div class="flex items-center justify-between gap-2">
      <button
        type="button"
        class="flex min-w-0 items-center gap-1.5 text-left"
        onclick={() => ontoggle()}
        aria-expanded={expanded}
      >
        <Icon
          name="chevronRight"
          size={11}
          class="shrink-0 text-faint transition-transform {expanded ? 'rotate-90' : ''}"
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
        class:max-h-44={!expanded}
        class:overflow-hidden={!expanded}
      >
        <AnsiOutput
          output={block.output}
          maxHeight={expanded ? 420 : 150}
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
          onclick={() => block.command && oncopy(`cmd-${block.index}`, block.command)}
        >
          <Icon name={copied === `cmd-${block.index}` ? "check" : "copy"} size={11} />
          Copy
        </button>
        <button
          type="button"
          class="flex items-center gap-1 rounded border border-line bg-panel px-1.5 py-0.5 text-[10px] text-muted transition-colors hover:bg-hover hover:text-fg"
          onclick={() => block.command && onrerun(block.command)}
        >
          <Icon name="zap" size={11} />
          Run
        </button>
      {/if}
      {#if block.output}
        <button
          type="button"
          class="flex items-center gap-1 rounded border border-line bg-panel px-1.5 py-0.5 text-[10px] text-muted transition-colors hover:bg-hover hover:text-fg"
          onclick={() => block.output && oncopy(`out-${block.index}`, block.output)}
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
