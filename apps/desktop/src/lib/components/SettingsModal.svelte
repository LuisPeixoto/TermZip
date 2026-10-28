<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";
  import { app } from "../stores/app.svelte";
  import { shortcutLabel } from "../shortcuts";
  import { api, type Settings } from "../api";
  import { workspace } from "../stores/workspace.svelte";

  let draft: Settings = $state({
    theme: "dark",
    fontSize: 14,
    fontFamily: null,
    scrollbackLines: 10000,
    shell: null,
    workingDirectory: null,
    restoreSession: true,
    defaultProfile: null,
    profiles: [],
  });
  let saving = $state(false);
  let dialog: HTMLDivElement | undefined = $state();

  const SCROLLBACK_OPTIONS = [1000, 5000, 10000, 50000, 100000, 200000];

  const heading = "mb-2 block text-[11px] font-semibold uppercase tracking-wide text-muted";
  const field =
    "mono w-full rounded-lg border border-line bg-panel-2 px-3 py-2 text-[12px] outline-none focus:border-accent";

  $effect(() => {
    if (app.settingsOpen) {
      draft = { ...app.settings };
    }
  });

  onMount(() => dialog?.focus());

  async function save() {
    saving = true;
    try {
      await app.updateSettings(draft);
      // Turning restore off must also forget what was saved, so write once now: the backend deletes
      // the file when restore is off and writes it when it is on.
      void api.saveWorkspace(workspace.snapshot()).catch(() => {});
      app.closeSettings();
      app.toast(
        "ok",
        `Preferences saved. Restart the shell (${shortcutLabel("restart", app.platform)}) to apply shell changes`,
      );
    } catch (error) {
      app.toast("err", `Could not save preferences: ${error}`);
    } finally {
      saving = false;
    }
  }
</script>

<div
  class="fixed inset-0 z-40 grid place-items-center bg-black/40 p-4"
  role="presentation"
  onclick={(event) => event.target === event.currentTarget && app.closeSettings()}
>
  <div
    bind:this={dialog}
    class="shadow-pop flex max-h-[calc(100vh-2rem)] w-full max-w-[420px] flex-col rounded-xl border border-line bg-panel outline-none"
    role="dialog"
    aria-modal="true"
    aria-labelledby="preferences-title"
    tabindex="-1"
  >
    <div class="flex shrink-0 items-center justify-between border-b border-line px-4 py-3">
      <h2 id="preferences-title" class="text-[13px] font-semibold">Preferences</h2>
      <button
        type="button"
        class="grid size-6 place-items-center rounded text-faint transition-colors hover:bg-hover hover:text-fg"
        onclick={() => app.closeSettings()}
        aria-label="Close preferences"
      >
        <Icon name="x" size={14} />
      </button>
    </div>

    <div class="min-h-0 flex-1 space-y-5 overflow-y-auto p-4">
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
        <p class="mt-1 text-[10.5px] text-faint">
          Any installed monospace family, e.g. “JetBrains Mono”.
        </p>
      </section>

      <section>
        <label for="pref-scrollback" class={heading}>Scrollback</label>
        <select id="pref-scrollback" class={field} bind:value={draft.scrollbackLines}>
          {#each SCROLLBACK_OPTIONS as option (option)}
            <option value={option}>{option.toLocaleString()} lines</option>
          {/each}
        </select>
      </section>

      <section>
        <label class="flex items-start gap-2.5 text-[12px]">
          <input
            type="checkbox"
            bind:checked={draft.restoreSession}
            class="mt-0.5 size-3.5 accent-accent"
          />
          <span>
            <span class="font-medium text-fg">Restore tabs and folders on launch</span>
            <span class="mt-0.5 block text-[10.5px] text-faint">
              Reopens your tabs, splits and the folder each shell was in. Terminal output is never saved.
            </span>
          </span>
        </label>
      </section>

      <section>
        <label for="pref-shell" class={heading}>Shell path</label>
        <input
          id="pref-shell"
          type="text"
          list="termzip-shell-paths"
          placeholder="System default shell"
          class={field}
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
        <label for="pref-directory" class={heading}>Working directory</label>
        <input
          id="pref-directory"
          type="text"
          placeholder="System default"
          class={field}
          value={draft.workingDirectory ?? ""}
          oninput={(event) => {
            const raw = event.currentTarget.value.trim();
            draft = { ...draft, workingDirectory: raw || null };
          }}
        />
      </section>
    </div>

    <div class="flex shrink-0 items-center justify-end gap-2 border-t border-line px-4 py-3">
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
