<script lang="ts">
  import Icon from "./Icon.svelte";
  import { app } from "../stores/app.svelte";
  import type { Settings } from "../api";

  let draft: Settings = $state({
    theme: "dark",
    fontSize: 14,
    fontFamily: null,
    scrollbackLines: 10000,
    shell: null,
    workingDirectory: null,
  });
  let saving = $state(false);

  const SCROLLBACK_OPTIONS = [1000, 5000, 10000, 50000, 100000, 200000];

  $effect(() => {
    if (app.settingsOpen) {
      draft = { ...app.settings };
    }
  });

  async function save() {
    saving = true;
    try {
      await app.updateSettings(draft);
      app.closeSettings();
      app.toast("ok", "Preferences saved — restart the shell (⌘R) to apply shell changes");
    } catch (error) {
      app.toast("err", `Could not save preferences: ${error}`);
    } finally {
      saving = false;
    }
  }
</script>

<div
  class="fixed inset-0 z-40 grid place-items-start justify-center bg-black/40 pt-24"
  role="presentation"
  onclick={(event) => event.target === event.currentTarget && app.closeSettings()}
>
  <div
    class="shadow-pop w-[420px] rounded-xl border border-line bg-panel"
    role="dialog"
    aria-label="Preferences"
  >
    <div class="flex items-center justify-between border-b border-line px-4 py-3">
      <h2 class="text-[13px] font-semibold">Preferences</h2>
      <button
        type="button"
        class="grid size-6 place-items-center rounded text-faint transition-colors hover:bg-hover hover:text-fg"
        onclick={() => app.closeSettings()}
      >
        <Icon name="x" size={14} />
      </button>
    </div>

    <div class="space-y-5 p-4">
      <section>
        <h3 class="mb-2 text-[11px] font-semibold uppercase tracking-wide text-muted">Theme</h3>
        <div class="flex gap-1 rounded-lg border border-line bg-panel-2 p-1">
          {#each ["dark", "light"] as option (option)}
            <button
              type="button"
              class="flex-1 rounded-md px-3 py-1.5 text-[12px] transition-colors
                {draft.theme === option ? 'bg-elevated font-medium text-fg' : 'text-muted'}"
              onclick={() => (draft = { ...draft, theme: option as Settings["theme"] })}
            >
              {option === "dark" ? "Dark" : "Light"}
            </button>
          {/each}
        </div>
      </section>

      <section>
        <h3 class="mb-2 text-[11px] font-semibold uppercase tracking-wide text-muted">
          Font size · <span class="text-fg">{Math.round(draft.fontSize)}px</span>
        </h3>
        <input
          type="range"
          min="10"
          max="28"
          step="1"
          bind:value={draft.fontSize}
          class="w-full accent-accent"
        />
      </section>

      <section>
        <h3 class="mb-2 text-[11px] font-semibold uppercase tracking-wide text-muted">Font family</h3>
        <input
          type="text"
          placeholder="System monospace default"
          class="mono w-full rounded-lg border border-line bg-panel-2 px-3 py-2 text-[12px] outline-none focus:border-accent"
          value={draft.fontFamily ?? ""}
          oninput={(event) => {
            const raw = event.currentTarget.value.trim();
            draft = { ...draft, fontFamily: raw || null };
          }}
        />
        <p class="mt-1 text-[10.5px] text-faint">
          Any installed monospace family, e.g. “JetBrains Mono”.
        </p>
      </section>

      <section>
        <h3 class="mb-2 text-[11px] font-semibold uppercase tracking-wide text-muted">Scrollback</h3>
        <select
          class="mono w-full rounded-lg border border-line bg-panel-2 px-3 py-2 text-[12px] outline-none focus:border-accent"
          bind:value={draft.scrollbackLines}
        >
          {#each SCROLLBACK_OPTIONS as option (option)}
            <option value={option}>{option.toLocaleString()} lines</option>
          {/each}
        </select>
      </section>

      <section>
        <h3 class="mb-2 text-[11px] font-semibold uppercase tracking-wide text-muted">Shell path</h3>
        <input
          type="text"
          list="termzip-shell-paths"
          placeholder="System default shell"
          class="mono w-full rounded-lg border border-line bg-panel-2 px-3 py-2 text-[12px] outline-none focus:border-accent"
          value={draft.shell ?? ""}
          oninput={(event) => {
            const raw = event.currentTarget.value.trim();
            draft = { ...draft, shell: raw || null };
          }}
        />
        <datalist id="termzip-shell-paths">
          {#each app.shells as shell (shell.path)}
            <option value={shell.path}>{shell.label}{shell.default ? " · default" : ""}</option>
          {/each}
        </datalist>
        <p class="mt-1 text-[10.5px] text-faint">
          Integrated shells: zsh, bash, fish, PowerShell. Other shells run without block capture.
        </p>
      </section>

      <section>
        <h3 class="mb-2 text-[11px] font-semibold uppercase tracking-wide text-muted">
          Working directory
        </h3>
        <input
          type="text"
          placeholder="System default"
          class="mono w-full rounded-lg border border-line bg-panel-2 px-3 py-2 text-[12px] outline-none focus:border-accent"
          value={draft.workingDirectory ?? ""}
          oninput={(event) => {
            const raw = event.currentTarget.value.trim();
            draft = { ...draft, workingDirectory: raw || null };
          }}
        />
      </section>
    </div>

    <div class="flex items-center justify-end gap-2 border-t border-line px-4 py-3">
      <button
        type="button"
        class="rounded-lg border border-line bg-panel-2 px-3 py-1.5 text-[12px] text-muted transition-colors hover:bg-hover hover:text-fg"
        onclick={() => app.closeSettings()}
      >
        Cancel
      </button>
      <button
        type="button"
        class="rounded-lg bg-accent px-3 py-1.5 text-[12px] font-semibold text-white transition-opacity hover:opacity-90 disabled:opacity-40"
        disabled={saving}
        onclick={() => void save()}
      >
        {saving ? "Saving…" : "Save"}
      </button>
    </div>
  </div>
</div>