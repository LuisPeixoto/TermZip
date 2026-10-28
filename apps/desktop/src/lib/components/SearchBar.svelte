<script lang="ts">
  import Icon from "./Icon.svelte";
  import type { SearchAddon } from "@xterm/addon-search";
  import { app } from "../stores/app.svelte";
  import type { Pane } from "../stores/pane.svelte";
  import { NO_RESULTS, describeResults, matchColors, regexProblem } from "../terminal/search";

  let { pane, addon }: { pane: Pane; addon: SearchAddon } = $props();

  let term = $state("");
  let caseSensitive = $state(false);
  let regex = $state(false);
  let input: HTMLInputElement | undefined = $state();

  const problem = $derived(regex ? regexProblem(term) : null);
  const summary = $derived(problem ? "Invalid expression" : describeResults(pane.searchResults, term !== ""));

  const options = () => ({
    regex,
    caseSensitive,
    decorations: matchColors(app.settings.theme === "dark"),
  });

  function find(direction: "next" | "previous", incremental = false) {
    if (!term || problem) return;
    if (direction === "next") addon.findNext(term, { ...options(), incremental });
    else addon.findPrevious(term, options());
  }

  // Every ⌘F brings the box back to the keyboard with its text selected, ready to be replaced.
  $effect(() => {
    pane.searchFocus;
    input?.focus();
    input?.select();
  });

  // Typing, and either toggle, search again from where the selection is.
  $effect(() => {
    term;
    caseSensitive;
    regex;
    if (!term || problem) {
      addon.clearDecorations();
      pane.searchResults = NO_RESULTS;
      return;
    }
    find("next", true);
  });

  function close() {
    addon.clearDecorations();
    pane.closeSearch();
    pane.controller?.focus();
  }

  const toggle =
    "grid h-6 min-w-6 place-items-center rounded px-1 text-[10.5px] font-semibold transition-colors";
</script>

<div
  class="absolute right-4 top-2 z-20 flex items-center gap-1 rounded-lg border border-line bg-elevated p-1 shadow-pop"
  role="search"
  aria-label="Find in the terminal"
>
  <input
    bind:this={input}
    bind:value={term}
    type="text"
    spellcheck="false"
    placeholder="Find"
    aria-label="Find in the terminal"
    class="mono h-6 w-44 rounded border bg-panel px-2 text-[11.5px] text-fg outline-none placeholder:text-faint
      {problem ? 'border-err' : 'border-line focus:border-accent'}"
    onkeydown={(event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        find(event.shiftKey ? "previous" : "next");
      } else if (event.key === "Escape") {
        event.preventDefault();
        close();
      }
      // Keys typed here are for this box; the application shortcuts must not see Escape and friends.
      event.stopPropagation();
    }}
  />
  <span class="min-w-[64px] text-center text-[10.5px] {problem ? 'text-err' : 'text-faint'}" aria-live="polite">
    {summary}
  </span>
  <button
    type="button"
    class="{toggle} {caseSensitive ? 'bg-accent-soft text-accent' : 'text-muted hover:bg-hover hover:text-fg'}"
    title="Match case"
    aria-label="Match case"
    aria-pressed={caseSensitive}
    onclick={() => (caseSensitive = !caseSensitive)}
  >
    Aa
  </button>
  <button
    type="button"
    class="{toggle} mono {regex ? 'bg-accent-soft text-accent' : 'text-muted hover:bg-hover hover:text-fg'}"
    title="Regular expression"
    aria-label="Regular expression"
    aria-pressed={regex}
    onclick={() => (regex = !regex)}
  >
    .*
  </button>
  <button
    type="button"
    class="grid size-6 place-items-center rounded text-muted transition-colors hover:bg-hover hover:text-fg disabled:opacity-40"
    title="Previous match (Shift+Enter)"
    aria-label="Previous match"
    disabled={!term || !!problem}
    onclick={() => find("previous")}
  >
    <Icon name="chevronRight" size={12} stroke={2.2} class="-rotate-90" />
  </button>
  <button
    type="button"
    class="grid size-6 place-items-center rounded text-muted transition-colors hover:bg-hover hover:text-fg disabled:opacity-40"
    title="Next match (Enter)"
    aria-label="Next match"
    disabled={!term || !!problem}
    onclick={() => find("next")}
  >
    <Icon name="chevronRight" size={12} stroke={2.2} class="rotate-90" />
  </button>
  <button
    type="button"
    class="grid size-6 place-items-center rounded text-faint transition-colors hover:bg-hover hover:text-fg"
    title="Close (Escape)"
    aria-label="Close search"
    onclick={close}
  >
    <Icon name="x" size={12} stroke={2.2} />
  </button>
</div>
