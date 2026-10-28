<script lang="ts">
  import { onDestroy } from "svelte";
  import type { Settings } from "../../api";
  import { app } from "../../stores/app.svelte";
  import {
    ACTIONS,
    chordOf,
    chordSafety,
    formatChord,
    resolveBindings,
    type ActionInfo,
    type AppShortcut,
  } from "../../keybindings";
  import { assignChord, clearChord, filterActions, isCustomized, resetChord } from "../../shortcutEdit";
  import { field } from "./styles";

  let { draft = $bindable() }: { draft: Settings } = $props();

  let query = $state("");
  // The action waiting for a key press.
  let recording = $state<AppShortcut | null>(null);
  // What was said about the last change to an action: a refusal, a warning, or whose shortcut it took.
  let notes = $state<Record<string, { kind: "info" | "warn" | "error"; text: string }>>({});

  const bindings = $derived(resolveBindings(draft.keybindings, app.platform));
  const visible = $derived(filterActions(ACTIONS, query));
  const groups = $derived(
    ["General", "Tabs and panes", "Terminal"]
      .map((name) => ({ name, actions: visible.filter((action) => action.group === name) }))
      .filter((group) => group.actions.length > 0),
  );
  const customized = $derived(ACTIONS.some((action) => isCustomized(draft.keybindings, app.platform, action.id)));

  const isTab = (action: ActionInfo) => /^tab[1-9]$/.test(action.id);

  function setOverrides(overrides: Record<string, string>) {
    draft = { ...draft, keybindings: overrides };
  }

  function note(action: AppShortcut, kind: "info" | "warn" | "error", text: string) {
    notes = { ...notes, [action]: { kind, text } };
  }

  function clearNote(action: AppShortcut) {
    const { [action]: _gone, ...rest } = notes;
    notes = rest;
  }

  function stop() {
    recording = null;
    app.setRecordingShortcut(false);
  }

  function start(action: AppShortcut) {
    clearNote(action);
    recording = action;
    app.setRecordingShortcut(true);
  }

  // The key that is pressed while recording is the answer: it never reaches the shortcuts or the terminal.
  function onRecordKey(event: KeyboardEvent) {
    const action = recording;
    if (!action) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.repeat) return;

    const plain = !event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey;
    if (event.key === "Escape" && plain) return stop();
    if (plain && (event.key === "Backspace" || event.key === "Delete")) {
      setOverrides(clearChord(draft.keybindings, app.platform, action));
      note(action, "info", "No shortcut.");
      return stop();
    }
    const chord = chordOf(event);
    if (!chord) return; // a modifier on its own: wait for the key that goes with it

    const safety = chordSafety(chord, app.platform);
    if (safety.level === "refuse") {
      note(action, "error", safety.message);
      return stop();
    }
    const { overrides, displaced } = assignChord(draft.keybindings, app.platform, action, chord);
    setOverrides(overrides);
    if (safety.level === "warn") note(action, "warn", safety.message);
    else if (displaced) note(action, "info", `Taken from “${displaced.label}”, which now has no shortcut.`);
    stop();
  }

  $effect(() => {
    if (!recording) return;
    window.addEventListener("keydown", onRecordKey, true);
    return () => window.removeEventListener("keydown", onRecordKey, true);
  });

  onDestroy(() => app.setRecordingShortcut(false));

  function reset(action: AppShortcut) {
    const { overrides, displaced } = resetChord(draft.keybindings, app.platform, action);
    setOverrides(overrides);
    if (displaced) note(action, "info", `Taken back from “${displaced.label}”, which now has no shortcut.`);
    else clearNote(action);
  }

  const small =
    "rounded-md border border-line px-2 py-1 text-[10.5px] text-muted transition-colors hover:bg-hover hover:text-fg disabled:opacity-40";
</script>

{#snippet row(action: ActionInfo)}
  <li class="px-3 py-2">
    <div class="flex items-center gap-2 text-[12px]">
      <span class="min-w-0 flex-1 truncate text-fg">{action.label}</span>
      {#if isCustomized(draft.keybindings, app.platform, action.id)}
        <span class="shrink-0 rounded bg-accent-soft px-1.5 py-0.5 text-[10px] font-medium text-accent">changed</span>
      {/if}
      <button
        type="button"
        class="mono min-w-[88px] shrink-0 rounded-md border px-2 py-1 text-center text-[11px] transition-colors
          {recording === action.id ? 'border-accent text-accent' : 'border-line text-fg hover:bg-hover'}"
        aria-label="Shortcut for {action.label}: {recording === action.id
          ? 'press the new keys'
          : formatChord(bindings[action.id], app.platform) || 'none'}. Press to change."
        onclick={() => (recording === action.id ? stop() : start(action.id))}
      >
        {#if recording === action.id}
          Press keys…
        {:else if bindings[action.id]}
          {formatChord(bindings[action.id], app.platform)}
        {:else}
          <span class="text-faint">Not set</span>
        {/if}
      </button>
      <button
        type="button"
        class={small}
        disabled={!isCustomized(draft.keybindings, app.platform, action.id)}
        onclick={() => reset(action.id)}
      >
        Reset
      </button>
    </div>
    {#if recording === action.id}
      <p class="mt-1 text-[10.5px] text-faint">Press the keys. Esc cancels, Backspace removes the shortcut.</p>
    {:else if notes[action.id]}
      <p
        class="mt-1 text-[10.5px] {notes[action.id].kind === 'error'
          ? 'text-err'
          : notes[action.id].kind === 'warn'
            ? 'text-warn'
            : 'text-muted'}"
        role={notes[action.id].kind === "info" ? "status" : "alert"}
      >
        {notes[action.id].text}
      </p>
    {/if}
  </li>
{/snippet}

<div class="space-y-4">
  <div class="flex items-center gap-2">
    <input
      type="search"
      bind:value={query}
      placeholder="Search the actions"
      aria-label="Search the actions"
      class="{field} flex-1"
    />
    <button
      type="button"
      class={small}
      disabled={!customized}
      onclick={() => {
        setOverrides({});
        notes = {};
      }}
    >
      Reset all
    </button>
  </div>

  {#each groups as group (group.name)}
    {@const tabs = group.actions.filter(isTab)}
    {@const others = group.actions.filter((action) => !isTab(action))}
    <section>
      <h4 class="mb-2 block text-[11px] font-semibold uppercase tracking-wide text-muted">{group.name}</h4>
      <ul class="divide-y divide-line rounded-lg border border-line bg-panel-2">
        {#each others as action (action.id)}
          {@render row(action)}
        {/each}
        {#if tabs.length > 0}
          <li>
            <details open={query.trim().length > 0}>
              <summary class="cursor-default px-3 py-2 text-[12px] text-muted hover:text-fg">
                Go to tab 1 to 9 ({tabs.length} shortcuts)
              </summary>
              <ul class="divide-y divide-line border-t border-line">
                {#each tabs as action (action.id)}
                  {@render row(action)}
                {/each}
              </ul>
            </details>
          </li>
        {/if}
      </ul>
    </section>
  {:else}
    <p class="text-[12px] text-faint">No action matches “{query}”.</p>
  {/each}

  <p class="text-[10.5px] leading-relaxed text-faint">
    {app.platform === "macos"
      ? "Shortcuts use ⌘ by default, which no shell uses."
      : "Shortcuts use Ctrl+Shift by default, because plain Ctrl keys belong to the shell."}
    A shortcut needs Ctrl, Alt or ⌘ with a key, and the keys it takes are no longer sent to the terminal.
    Changes apply when you press Save.
  </p>
</div>
