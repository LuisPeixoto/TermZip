<script lang="ts">
  import { app } from "../../stores/app.svelte";
  import type { Settings } from "../../api";
  import { field, heading, hint } from "./styles";

  let { draft = $bindable() }: { draft: Settings } = $props();
</script>

<div class="space-y-5">
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
    <p class={hint}>Integrated shells: zsh, bash, fish, PowerShell. Other shells run without block capture.</p>
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
