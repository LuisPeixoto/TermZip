<script lang="ts">
  import { onMount, tick } from "svelte";
  import { highlight, rank } from "../fuzzy";
  import { focusTerminal } from "../focus";
  import { resolveBindings } from "../keybindings";
  import { paletteEntries, type PaletteEntry } from "../paletteEntries";
  import { runAction } from "../runAction";
  import { app } from "../stores/app.svelte";
  import { api, type SnippetInfo, type TaskInfo } from "../api";
  import { initialValues, missingValues } from "../snippets";
  import { pluginStore } from "../stores/plugins.svelte";
  import { themes } from "../stores/themes.svelte";
  import { workspace } from "../stores/workspace.svelte";

  let query = $state("");
  let active = $state(0);
  let input: HTMLInputElement | undefined = $state();
  // The snippets of the folder the active pane is in, read when the palette opens.
  let snippets = $state.raw<SnippetInfo[]>([]);
  // The tasks of the project the active pane is in, read at the same time.
  let tasks = $state.raw<TaskInfo[]>([]);
  // The form that asks for the values of a snippet; the list is hidden while it is open.
  let form = $state<{ snippet: SnippetInfo; values: Record<string, string> } | null>(null);
  let formElement: HTMLFormElement | undefined = $state();

  const entries = $derived(
    paletteEntries({
      platform: app.platform,
      bindings: resolveBindings(app.settings.keybindings, app.platform),
      themes: themes.entries,
      profiles: app.settings.profiles,
      tabs: workspace.tabs.map((tab) => tab.title),
      currentThemeId: app.theme?.id ?? null,
      blurSupported: app.blurSupported,
      snippets,
      tasks,
      scope: app.paletteScope,
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
        useSnippet: (snippet) => {
          if (snippet.parameters.length === 0) void insertSnippet(snippet, {});
          else form = { snippet, values: initialValues(snippet) };
        },
        typeTask: (task) => {
          const pane = workspace.activePane;
          if (!pane.session) {
            app.toast("warn", "The shell is not running yet");
            return;
          }
          // Only typed: the command waits at the prompt for you to read it and press Enter.
          pane.paste(task.command);
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

  onMount(() => {
    input?.focus();
    const pane = workspace.activePane;
    const folder = pane.cwd ?? pane.restoreDirectory ?? null;
    api
      .listSnippets(folder)
      .then((list) => (snippets = list))
      .catch((error) => app.toast("warn", `Could not load the snippets: ${error}`));
    // A project without tasks, or a folder that cannot be read, is not worth a message.
    api
      .listTasks(folder)
      .then((list) => (tasks = list))
      .catch(() => {});
  });

  // The first field of the form is ready to type in as soon as the form opens.
  $effect(() => {
    if (form) formElement?.querySelector("input")?.focus();
  });

  /** Puts a snippet at the prompt, as a paste: it is never run, the text waits for you to read it and press Enter. */
  async function insertSnippet(snippet: SnippetInfo, values: Record<string, string>) {
    const pane = workspace.activePane;
    if (!pane.session) {
      app.toast("warn", "The shell is not running yet");
      return;
    }
    try {
      pane.paste(await api.renderSnippet(snippet.id, values));
      focusTerminal();
    } catch (error) {
      app.toast("warn", String(error));
    }
  }

  const missing = $derived(form ? missingValues(form.snippet, form.values) : []);

  function submitForm(event: SubmitEvent) {
    event.preventDefault();
    if (!form || missing.length > 0) return;
    const { snippet } = form;
    const values = $state.snapshot(form.values);
    app.closePalette();
    // After the palette has gone, so the terminal that gets the text is the one that has the keyboard.
    setTimeout(() => void insertSnippet(snippet, values), 0);
  }

  function onFormKeydown(event: KeyboardEvent) {
    if (event.key !== "Escape") return;
    // Back to the list, not out of the palette.
    event.preventDefault();
    event.stopPropagation();
    form = null;
    void tick().then(() => input?.focus());
  }

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
    if (entry.keepsOpen) {
      entry.run();
      return;
    }
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
    <div class="flex min-h-0 flex-1 flex-col" class:hidden={form !== null}>
      <input
        bind:this={input}
        bind:value={query}
        type="text"
        class="mono w-full shrink-0 border-b border-line bg-transparent px-4 py-3 text-[13px] outline-none placeholder:text-faint"
        placeholder={app.paletteScope === "snippets" ? "Type a snippet" : "Type a command"}
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
          <li class="px-2.5 py-3 text-[12px] text-faint" role="presentation">
              {#if app.paletteScope === "snippets" && snippets.length === 0}
                There are no snippets for this folder. Add some in Preferences → Snippets.
              {:else}
                No {app.paletteScope === "snippets" ? "snippet" : "command"} matches “{query}”.
              {/if}
            </li>
        {/each}
      </ul>
    </div>
    {#if form}
      <!-- Escape is pressed in one of the fields or buttons inside; the form only receives it. -->
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <form bind:this={formElement} class="flex flex-col gap-3 p-4" onsubmit={submitForm} onkeydown={onFormKeydown}>
        <div>
          <p class="text-[13px] font-semibold">{form.snippet.name}</p>
          {#if form.snippet.description}
            <p class="mt-0.5 text-[11.5px] text-muted">{form.snippet.description}</p>
          {/if}
        </div>
        {#each form.snippet.parameters as parameter (parameter.name)}
          <label class="flex flex-col gap-1 text-[11.5px] text-muted">
            <span class="mono">{parameter.name}{parameter.default === null ? "" : " (optional)"}</span>
            <input
              type="text"
              class="mono w-full rounded-lg border border-line bg-panel-2 px-3 py-2 text-[12px] text-fg outline-none focus:border-accent"
              bind:value={form.values[parameter.name]}
              spellcheck="false"
              autocomplete="off"
            />
          </label>
        {/each}
        <p class="text-[10.5px] text-faint">The text is typed at the prompt. Nothing runs until you press Enter.</p>
        <div class="flex justify-end gap-2">
          <button
            type="button"
            class="rounded-md border border-line px-2.5 py-1 text-[11px] text-muted hover:bg-hover hover:text-fg"
            onclick={() => {
              form = null;
              void tick().then(() => input?.focus());
            }}
          >
            Back
          </button>
          <button
            type="submit"
            class="rounded-md bg-accent px-2.5 py-1 text-[11px] font-semibold text-white hover:opacity-90 disabled:opacity-40"
            disabled={missing.length > 0}
            title={missing.length > 0 ? `Fill in ${missing.join(", ")}` : undefined}
          >
            Type it
          </button>
        </div>
      </form>
    {/if}
  </div>
</div>
