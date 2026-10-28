<script lang="ts">
  import type { Settings } from "../../api";
  import { heading } from "./styles";

  let { draft = $bindable() }: { draft: Settings } = $props();
</script>

<div class="space-y-5">
  <section>
    <h3 id="pref-theme" class={heading}>Theme</h3>
    <div
      class="flex gap-1 rounded-lg border border-line bg-panel-2 p-1"
      role="group"
      aria-labelledby="pref-theme"
    >
      {#each ["dark", "light"] as option (option)}
        <button
          type="button"
          class="flex-1 rounded-md px-3 py-1.5 text-[12px] transition-colors
            {draft.theme === option ? 'bg-elevated font-medium text-fg' : 'text-muted'}"
          aria-pressed={draft.theme === option}
          onclick={() => (draft = { ...draft, theme: option as Settings["theme"] })}
        >
          {option === "dark" ? "Dark" : "Light"}
        </button>
      {/each}
    </div>
  </section>
</div>
