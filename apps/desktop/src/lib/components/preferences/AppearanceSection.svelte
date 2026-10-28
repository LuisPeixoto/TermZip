<script lang="ts">
  import type { Settings } from "../../api";
  import ThemeGallery from "./ThemeGallery.svelte";
  import { app } from "../../stores/app.svelte";
  import { LIMITS } from "../../settings";
  import { heading, hint } from "./styles";

  let { draft = $bindable() }: { draft: Settings } = $props();
</script>

<div class="space-y-5">
  <section>
    <h3 id="pref-theme" class={heading}>Appearance</h3>
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

  <section>
    <label for="pref-opacity" class={heading}>
      Window opacity · <span class="text-fg">{Math.round(draft.backgroundOpacity * 100)}%</span>
    </label>
    <input
      id="pref-opacity"
      type="range"
      min={LIMITS.backgroundOpacity.min}
      max={LIMITS.backgroundOpacity.max}
      step="0.05"
      bind:value={draft.backgroundOpacity}
      class="w-full accent-accent"
    />
    {#if app.blurSupported}
      <label class="mt-2.5 flex items-center gap-2.5 text-[12px]">
        <input
          type="checkbox"
          bind:checked={draft.backgroundBlur}
          disabled={draft.backgroundOpacity >= 1}
          class="size-3.5 accent-accent"
        />
        <span class="font-medium text-fg">Blur what shows through</span>
      </label>
    {:else if draft.backgroundOpacity < 1}
      <p class={hint}>
        The desktop shows through if your window manager composites windows. Blur is not available on this system.
      </p>
    {/if}
  </section>

  <section>
    <h3 class={heading}>Themes</h3>
    <ThemeGallery bind:draft />
  </section>
</div>
