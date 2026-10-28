<script lang="ts">
  import type { Settings } from "../../api";
  import { field, heading, hint } from "./styles";

  let { draft = $bindable() }: { draft: Settings } = $props();

  const SCROLLBACK_OPTIONS = [1000, 5000, 10000, 50000, 100000, 200000];
</script>

<div class="space-y-5">
  <section>
    <label for="pref-font-size" class={heading}>
      Font size · <span class="text-fg">{Math.round(draft.fontSize)}px</span>
    </label>
    <input
      id="pref-font-size"
      type="range"
      min="10"
      max="28"
      step="1"
      bind:value={draft.fontSize}
      class="w-full accent-accent"
    />
  </section>

  <section>
    <label for="pref-font-family" class={heading}>Font family</label>
    <input
      id="pref-font-family"
      type="text"
      placeholder="System monospace default"
      class={field}
      value={draft.fontFamily ?? ""}
      oninput={(event) => {
        const raw = event.currentTarget.value.trim();
        draft = { ...draft, fontFamily: raw || null };
      }}
    />
    <p class={hint}>Any installed monospace family, e.g. “JetBrains Mono”.</p>
  </section>

  <section>
    <label for="pref-scrollback" class={heading}>Scrollback</label>
    <select id="pref-scrollback" class={field} bind:value={draft.scrollbackLines}>
      {#each SCROLLBACK_OPTIONS as option (option)}
        <option value={option}>{option.toLocaleString()} lines</option>
      {/each}
    </select>
  </section>
</div>
