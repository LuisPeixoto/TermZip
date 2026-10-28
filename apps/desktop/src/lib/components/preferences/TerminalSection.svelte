<script lang="ts">
  import TerminalSample from "./TerminalSample.svelte";
  import type { CursorStyle, Settings } from "../../api";
  import { LIMITS } from "../../settings";
  import { field, heading, hint } from "./styles";

  let { draft = $bindable() }: { draft: Settings } = $props();

  const SCROLLBACK_OPTIONS = [1000, 5000, 10000, 50000, 100000, 200000];
  const CURSORS: { id: CursorStyle; label: string }[] = [
    { id: "block", label: "Block" },
    { id: "bar", label: "Bar" },
    { id: "underline", label: "Underline" },
  ];
</script>

<div class="space-y-5">
  <!-- Stays in view while the controls below it are scrolled through. -->
  <div class="sticky -top-4 z-10 -mx-4 -mt-4 bg-panel px-4 pb-3 pt-4">
    <TerminalSample
      fontSize={draft.fontSize}
      fontFamily={draft.fontFamily}
      lineHeight={draft.lineHeight}
      letterSpacing={draft.letterSpacing}
      cursorStyle={draft.cursorStyle}
      cursorBlink={draft.cursorBlink}
    />
  </div>

  <section>
    <label for="pref-font-size" class={heading}>
      Font size · <span class="text-fg">{Math.round(draft.fontSize)}px</span>
    </label>
    <input
      id="pref-font-size"
      type="range"
      min={LIMITS.fontSize.min}
      max={LIMITS.fontSize.max}
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
    <label for="pref-line-height" class={heading}>
      Line height · <span class="text-fg">{draft.lineHeight.toFixed(2)}</span>
    </label>
    <input
      id="pref-line-height"
      type="range"
      min={LIMITS.lineHeight.min}
      max={LIMITS.lineHeight.max}
      step="0.05"
      bind:value={draft.lineHeight}
      class="w-full accent-accent"
    />
  </section>

  <section>
    <label for="pref-letter-spacing" class={heading}>
      Letter spacing · <span class="text-fg">{draft.letterSpacing.toFixed(1)}px</span>
    </label>
    <input
      id="pref-letter-spacing"
      type="range"
      min={LIMITS.letterSpacing.min}
      max={LIMITS.letterSpacing.max}
      step="0.5"
      bind:value={draft.letterSpacing}
      class="w-full accent-accent"
    />
  </section>

  <section>
    <h3 id="pref-cursor" class={heading}>Cursor</h3>
    <div
      class="flex gap-1 rounded-lg border border-line bg-panel-2 p-1"
      role="group"
      aria-labelledby="pref-cursor"
    >
      {#each CURSORS as option (option.id)}
        <button
          type="button"
          class="flex-1 rounded-md px-3 py-1.5 text-[12px] transition-colors
            {draft.cursorStyle === option.id ? 'bg-elevated font-medium text-fg' : 'text-muted'}"
          aria-pressed={draft.cursorStyle === option.id}
          onclick={() => (draft = { ...draft, cursorStyle: option.id })}
        >
          {option.label}
        </button>
      {/each}
    </div>
    <label class="mt-2.5 flex items-center gap-2.5 text-[12px]">
      <input type="checkbox" bind:checked={draft.cursorBlink} class="size-3.5 accent-accent" />
      <span class="font-medium text-fg">Blink the cursor</span>
    </label>
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
