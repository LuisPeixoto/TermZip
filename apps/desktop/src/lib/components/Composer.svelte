<script lang="ts">
  import Icon from "./Icon.svelte";
  import { app } from "../stores/app.svelte";
  import { api } from "../api";

  let value = $state("");
  let input: HTMLTextAreaElement | undefined = $state();
  let history: string[] = $state([]);
  let historyIndex: number | null = $state(null);
  const MAX_HISTORY = 200;

  $effect(() => {
    function onComposer(event: Event) {
      const detail = (event as CustomEvent<{ command: string; execute: boolean }>).detail;
      value = detail.command;
      input?.focus();
      if (detail.execute) void submit();
    }
    window.addEventListener("termzip:composer", onComposer);
    return () => window.removeEventListener("termzip:composer", onComposer);
  });

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      void submit();
      return;
    }
    if (event.key === "ArrowUp" && history.length > 0 && !event.altKey) {
      event.preventDefault();
      const next = historyIndex == null ? history.length - 1 : Math.max(0, historyIndex - 1);
      historyIndex = next;
      value = history[next] ?? value;
      return;
    }
    if (event.key === "ArrowDown" && historyIndex != null) {
      event.preventDefault();
      const next = historyIndex + 1;
      if (next >= history.length) {
        historyIndex = null;
        value = "";
      } else {
        historyIndex = next;
        value = history[next] ?? value;
      }
      return;
    }
  }

  async function submit(): Promise<void> {
    const command = value.trim();
    if (!command) return;
    value = "";
    historyIndex = null;
    if (history.at(-1) !== command) {
      history.push(command);
      if (history.length > MAX_HISTORY) history.shift();
    }
    await sendRaw(`${command}\r`);
  }

  async function sendRaw(payload: string): Promise<void> {
    const session = app.session;
    if (!session) {
      app.toast("warn", "The shell is not running yet");
      return;
    }
    if (app.exited) {
      app.toast("err", "The shell has exited — press ⌘R to restart");
      return;
    }
    try {
      await api.writeSession(session.id, new TextEncoder().encode(payload));
    } catch (error) {
      app.toast("err", `Could not send the command: ${error}`);
    }
  }
</script>

<div class="shrink-0 px-3 pb-2">
  <div
    class="flex items-end gap-2 rounded-xl border bg-panel px-3 py-2 transition-colors
      {app.exited ? 'border-line' : 'border-accent'}"
  >
    <span
      class="mb-0.5 grid size-7 shrink-0 place-items-center rounded-lg bg-accent-soft text-accent"
      aria-hidden="true"
    >
      <Icon name="chevronRight" size={14} stroke={2.4} />
    </span>

    <textarea
      bind:this={input}
      bind:value
      rows="1"
      spellcheck="false"
      autocomplete="off"
      class="mono selectable max-h-28 min-h-[24px] flex-1 resize-none border-0 bg-transparent p-0 text-[12.5px] leading-6 text-fg outline-none placeholder:text-faint"
      placeholder={app.exited
        ? "Shell exited — press ⌘R to restart"
        : "Run a command…  (Enter to execute, Shift+Enter for a new line)"}
      onkeydown={onKeydown}
    ></textarea>

    <button
      type="button"
      class="mb-0.5 flex h-7 items-center gap-1.5 rounded-lg bg-accent px-2.5 text-[11.5px] font-semibold text-white transition-opacity hover:opacity-90 disabled:opacity-40"
      disabled={!value.trim() || app.exited}
      onclick={() => void submit()}
    >
      <Icon name="send" size={12} />
      Run
    </button>
  </div>
</div>