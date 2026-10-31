<script lang="ts">
  import { onMount } from "svelte";
  import { highlight, rank } from "../fuzzy";
  import { focusTerminal } from "../focus";
  import { resolveBindings } from "../keybindings";
  import { paletteEntries, type PaletteEntry } from "../paletteEntries";
  import { runAction } from "../runAction";
  import { app } from "../stores/app.svelte";
  import { api } from "../api";
  import { pluginStore } from "../stores/plugins.svelte";
  import { themes } from "../stores/themes.svelte";
  import { workspace } from "../stores/workspace.svelte";

  let query = $state("");
  let active = $state(0);
  let input: HTMLInputElement | undefined = $state();

  const entries = $derived(
    paletteEntries({
      platform: app.platform,
      bindings: resolveBindings(app.settings.keybindings, app.platform),
      themes: themes.entries,
      profiles: app.settings.profiles,
      tabs: workspace.tabs.map((tab) => tab.title),
      currentThemeId: app.theme?.id ?? null,
      blurSupported: app.blurSupported,
      plugins: pluginStore.running.flatMap((plugin) =>
        plugin.manifest.commands.map((command) => ({
          plugin: plugin.manifest.id,
          pluginName: plugin.manifest.name,
          id: command.id,
          title: command.title,
        })),
      ),
      commands: {
        run: runAction,
        goToTab: (position) => {
          workspace.activateAt(position);
          focusTerminal();
        },
        newTab: (profileId) => {
          if (!workspace.newTab(profileId)) app.toast("warn", "That is the most tabs a window can hold");
          focusTerminal();
        },
        useTheme: (theme) => {
          const choice =
            theme.appearance === "dark"
              ? { theme: "dark" as const, darkTheme: theme.id }
              : { theme: "light" as const, lightTheme: theme.id };
          void app.updateSettings(choice).catch((error) => app.toast("err", `Could not change the theme: ${error}`));
        },
        runPluginCommand: (plugin, command) => {
          const pane = workspace.activePane;
          if (!pane.session) {
            app.toast("warn", "The shell is not running yet");
            return;
          }
          // The answer arrives on the plugin channel and names this terminal, so typing goes where it was run.
          void api
            .runPluginCommand(plugin, command, pane.cwd ?? pane.restoreDirectory, pane.session.id)
            .catch((error) => app.toast("warn", String(error)));
          focusTerminal();
        },
        toggleAppearance: () => void app.toggleTheme(),
        toggleBlur: () => {
          void app
            .updateSettings({ backgroundBlur: !app.settings.backgroundBlur })
            .catch((error) => app.toast("err", `Could not change the blur: ${error}`));
        },
      },
    }),
  );
  const results = $derived(
    rank(entries, query, (entry) => entry.title, (entry) => entry.id, app.recentActions),
  );

  onMount(() => input?.focus());

  // A new search starts at the top, and the highlighted entry is kept in view as the arrows move it.
  $effect(() => {
    query;
    active = 0;
  });
  $effect(() => {
    document.getElementById(`palette-option-${active}`)?.scrollIntoView({ block: "nearest" });
  });

  function choose(entry: PaletteEntry) {
    app.rememberAction(entry.id);
    app.closePalette();
    // After the palette has gone, so what the command focuses is not taken back by it.
    setTimeout(() => entry.run(), 0);
  }

  function onKeydown(event: KeyboardEvent) {
    const count = results.length;
    if (event.key === "ArrowDown" && count > 0) {
      event.preventDefault();
      active = (active + 1) % count;
    } else if (event.key === "ArrowUp" && count > 0) {
      event.preventDefault();
      active = (active - 1 + count) % count;
    } else if (event.key === "Enter") {
      event.preventDefault();
      const chosen = results[active];
      if (chosen) choose(chosen.item);
    } else if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      app.closePalette();
    }
  }
</script>

<div
  class="fixed inset-0 z-40 flex items-start justify-center bg-black/30 p-4 pt-[12vh]"
  role="presentation"
  onclick={(event) => event.target === event.currentTarget && app.closePalette()}
>
  <div
    class="shadow-pop flex max-h-[min(440px,calc(100vh-8rem))] w-full max-w-[560px] flex-col overflow-hidden rounded-xl border border-line bg-panel"
    role="dialog"
    aria-modal="true"
    aria-label="Command palette"
  >
    <input
      bind:this={input}
      bind:value={query}
      type="text"
      class="mono w-full shrink-0 border-b border-line bg-transparent px-4 py-3 text-[13px] outline-none placeholder:text-faint"
      placeholder="Type a command"
      spellcheck="false"
      autocomplete="off"
      role="combobox"
      aria-expanded="true"
      aria-controls="palette-list"
      aria-activedescendant={results.length > 0 ? `palette-option-${active}` : undefined}
      aria-label="Command"
      onkeydown={onKeydown}
    />
    <ul id="palette-list" role="listbox" aria-label="Commands" class="min-h-0 flex-1 overflow-y-auto p-1.5">
      {#each results as result, index (result.item.id)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <li
          id="palette-option-{index}"
          role="option"
          aria-selected={index === active}
          class="flex cursor-default items-center gap-3 rounded-lg px-2.5 py-1.5 text-[12.5px]
            {index === active ? 'bg-hover text-fg' : 'text-muted'}"
          onmousemove={() => (active = index)}
          onclick={() => choose(result.item)}
        >
          <span class="min-w-0 flex-1 truncate">
            {#each highlight(result.item.title, result.ranges) as part, at (at)}
              <span class={part.hit ? 'font-semibold text-accent' : index === active ? 'text-fg' : ''}>{part.text}</span>
            {/each}
          </span>
          {#if result.item.detail}
            <kbd class="mono shrink-0 text-[10.5px] text-faint">{result.item.detail}</kbd>
          {/if}
        </li>
      {:else}
        <li class="px-2.5 py-3 text-[12px] text-faint" role="presentation">No command matches “{query}”.</li>
      {/each}
    </ul>
  </div>
</div>
