<script lang="ts">
  import { onMount } from "svelte";
  import type { Component } from "svelte";
  import Icon from "./Icon.svelte";
  import AppearanceSection from "./preferences/AppearanceSection.svelte";
  import GeneralSection from "./preferences/GeneralSection.svelte";
  import ProfilesSection from "./preferences/ProfilesSection.svelte";
  import TerminalSection from "./preferences/TerminalSection.svelte";
  import { app } from "../stores/app.svelte";
  import { shortcutLabel } from "../shortcuts";
  import { api, type Settings } from "../api";
  import { defaultSettings } from "../settings";
  import { workspace } from "../stores/workspace.svelte";

  const SECTIONS: { id: string; label: string; component: Component<{ draft: Settings }> }[] = [
    { id: "general", label: "General", component: GeneralSection },
    { id: "appearance", label: "Appearance", component: AppearanceSection },
    { id: "terminal", label: "Terminal", component: TerminalSection },
    { id: "profiles", label: "Profiles", component: ProfilesSection },
  ];

  let draft: Settings = $state(defaultSettings());
  let section = $state(SECTIONS[0].id);
  let saving = $state(false);
  let saveError = $state<string | null>(null);
  let dialog: HTMLDivElement | undefined = $state();

  const Active = $derived(SECTIONS.find((entry) => entry.id === section)?.component ?? GeneralSection);

  $effect(() => {
    if (app.settingsOpen) {
      draft = { ...app.settings };
    }
  });

  onMount(() => dialog?.focus());

  // Arrow keys move between the sections, as in any tab list; only the selected one is a tab stop.
  function onNavKeydown(event: KeyboardEvent) {
    const step = event.key === "ArrowDown" ? 1 : event.key === "ArrowUp" ? -1 : 0;
    if (step === 0) return;
    event.preventDefault();
    const index = SECTIONS.findIndex((entry) => entry.id === section);
    const next = SECTIONS[(index + step + SECTIONS.length) % SECTIONS.length];
    section = next.id;
    document.getElementById(`pref-tab-${next.id}`)?.focus();
  }

  async function save() {
    saving = true;
    saveError = null;
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
      // The backend explains what is wrong with a profile in a sentence; show it where it can be read.
      saveError = String(error);
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
    class="shadow-pop flex h-[min(560px,calc(100vh-2rem))] w-full max-w-[720px] flex-col rounded-xl border border-line bg-panel outline-none"
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

    <div class="flex min-h-0 flex-1">
      <div
        class="flex w-[168px] shrink-0 flex-col gap-0.5 border-r border-line p-2"
        role="tablist"
        aria-orientation="vertical"
        aria-label="Preferences sections"
      >
        {#each SECTIONS as entry (entry.id)}
          <button
            type="button"
            id="pref-tab-{entry.id}"
            role="tab"
            aria-selected={section === entry.id}
            aria-controls="pref-panel"
            tabindex={section === entry.id ? 0 : -1}
            class="rounded-md px-2.5 py-1.5 text-left text-[12px] transition-colors
              {section === entry.id ? 'bg-elevated font-medium text-fg' : 'text-muted hover:bg-hover hover:text-fg'}"
            onclick={() => (section = entry.id)}
            onkeydown={onNavKeydown}
          >
            {entry.label}
          </button>
        {/each}
      </div>

      <div
        id="pref-panel"
        role="tabpanel"
        aria-labelledby="pref-tab-{section}"
        class="min-h-0 min-w-0 flex-1 overflow-y-auto p-4"
      >
        <Active bind:draft />
      </div>
    </div>

    {#if saveError}
      <p class="shrink-0 border-t border-line bg-err/10 px-4 py-2 text-[11.5px] text-err" role="alert">
        {saveError}
      </p>
    {/if}

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
