<script lang="ts">
  import Icon from "../Icon.svelte";
  import ThemePreview from "./ThemePreview.svelte";
  import { api, type Settings, type Theme } from "../../api";
  import { app } from "../../stores/app.svelte";
  import { themes } from "../../stores/themes.svelte";
  import { DEFAULT_DARK_THEME, DEFAULT_LIGHT_THEME, describeCredit, groupThemes } from "../../theme";
  import { heading } from "./styles";

  let { draft = $bindable() }: { draft: Settings } = $props();

  const groups = $derived.by(() => {
    const { dark, light } = groupThemes(themes.entries);
    return [
      { label: "Dark themes", entries: dark },
      { label: "Light themes", entries: light },
    ];
  });

  let busy = $state(false);
  let problem = $state<string | null>(null);
  // The theme whose delete is waiting for a second click.
  let confirming = $state<string | null>(null);

  const slotOf = (theme: Theme) => (theme.appearance === "dark" ? draft.darkTheme : draft.lightTheme);
  const inUse = (theme: Theme) => draft.theme === theme.appearance && slotOf(theme) === theme.id;

  /** Choosing a theme also switches to its appearance, so the choice is seen at once. */
  function use(theme: Theme) {
    confirming = null;
    draft =
      theme.appearance === "dark"
        ? { ...draft, theme: "dark", darkTheme: theme.id }
        : { ...draft, theme: "light", lightTheme: theme.id };
  }

  async function run(action: () => Promise<void>) {
    busy = true;
    problem = null;
    try {
      await action();
    } catch (error) {
      problem = String(error);
    } finally {
      busy = false;
    }
  }

  const importTheme = () =>
    run(async () => {
      const entry = await api.importTheme();
      if (!entry) return; // the dialog was cancelled
      await themes.load();
      use(entry.theme);
      app.toast("ok", `Imported “${entry.theme.name}”`);
    });

  const exportTheme = (theme: Theme) =>
    run(async () => {
      if (await api.exportTheme(theme.id)) app.toast("ok", `Exported “${theme.name}”`);
    });

  const deleteTheme = (theme: Theme) =>
    run(async () => {
      await api.deleteTheme(theme.id);
      await themes.load();
      confirming = null;
      // A slot that pointed at it goes back to TermZip's own instead of at nothing.
      if (draft.darkTheme === theme.id) draft = { ...draft, darkTheme: DEFAULT_DARK_THEME };
      if (draft.lightTheme === theme.id) draft = { ...draft, lightTheme: DEFAULT_LIGHT_THEME };
    });

  const small =
    "rounded-md border border-line px-2 py-1 text-[10.5px] text-muted transition-colors hover:bg-hover hover:text-fg disabled:opacity-40";
</script>

<div class="space-y-4">
  <div class="flex items-start justify-between gap-3">
    <p class="text-[10.5px] leading-relaxed text-faint">
      A theme colors the whole window. Each appearance remembers its own; the sun and moon button switches between them.
      Import iTerm2, Windows Terminal, Ghostty, Kitty or Alacritty themes.
    </p>
    <button type="button" class="{small} shrink-0" disabled={busy} onclick={() => void importTheme()}>
      Import…
    </button>
  </div>

  {#each groups as group (group.label)}
    <section>
      <h4 class={heading}>{group.label}</h4>
      <div class="grid grid-cols-2 gap-2.5">
        {#each group.entries as item (item.theme.id)}
          {@const theme = item.theme}
          <div
            class="overflow-hidden rounded-lg border bg-panel-2 transition-colors
              {inUse(theme) ? 'border-accent' : 'border-line'}"
          >
            <button
              type="button"
              class="block w-full text-left"
              aria-pressed={inUse(theme)}
              aria-label="Use {theme.name}"
              onclick={() => use(theme)}
            >
              <ThemePreview {theme} />
              <span class="flex items-center gap-1.5 px-2.5 pt-2 text-[12px] font-medium text-fg">
                <span class="min-w-0 truncate">{theme.name}</span>
                {#if inUse(theme)}
                  <span class="shrink-0 rounded bg-accent-soft px-1.5 py-0.5 text-[10px] font-medium text-accent">in use</span>
                {:else if slotOf(theme) === theme.id}
                  <span class="shrink-0 rounded bg-hover px-1.5 py-0.5 text-[10px] font-medium text-muted">
                    {theme.appearance === "dark" ? "dark" : "light"} choice
                  </span>
                {/if}
              </span>
              <span class="block truncate px-2.5 pb-1 text-[10.5px] text-faint">
                {describeCredit(theme) || "Imported"}
              </span>
            </button>
            <div class="flex items-center gap-1.5 px-2.5 pb-2 pt-1">
              <button type="button" class={small} disabled={busy} onclick={() => void exportTheme(theme)}>
                Export…
              </button>
              {#if !item.builtIn}
                {#if confirming === theme.id}
                  <button
                    type="button"
                    class="rounded-md border border-err/50 bg-err/10 px-2 py-1 text-[10.5px] text-err disabled:opacity-40"
                    disabled={busy}
                    onclick={() => void deleteTheme(theme)}
                  >
                    Delete for good?
                  </button>
                  <button type="button" class={small} onclick={() => (confirming = null)}>Keep</button>
                {:else}
                  <button
                    type="button"
                    class="{small} grid place-items-center"
                    aria-label="Delete {theme.name}"
                    title="Delete"
                    disabled={busy}
                    onclick={() => (confirming = theme.id)}
                  >
                    <Icon name="x" size={11} />
                  </button>
                {/if}
              {/if}
            </div>
          </div>
        {/each}
      </div>
    </section>
  {/each}

  {#if problem}
    <p class="rounded-lg border border-err/40 bg-err/10 px-3 py-2 text-[11px] text-err" role="alert">{problem}</p>
  {/if}

  {#if themes.problems.length > 0}
    <div class="rounded-lg border border-warn/40 bg-warn/10 px-3 py-2 text-[11px] text-warn">
      <p class="mb-1 font-medium">Some of your theme files were left out:</p>
      <ul class="list-disc space-y-0.5 pl-4">
        {#each themes.problems as sentence (sentence)}
          <li>{sentence}</li>
        {/each}
      </ul>
    </div>
  {/if}
</div>
