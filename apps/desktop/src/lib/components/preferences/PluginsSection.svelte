<script lang="ts">
  import { onMount } from "svelte";
  import { api, type CatalogItem, type Plugin, type Settings } from "../../api";
  import { app } from "../../stores/app.svelte";
  import { pluginStore } from "../../stores/plugins.svelte";
  import {
    catalogAction,
    describeAdds,
    describeAsk,
    describeSource,
    describeState,
    formatSize,
    permissionChips,
    permissionChipsOf,
    stateOf,
  } from "../../plugins";
  import { heading } from "./styles";

  // Plugins apply at once; only the choice to allow the catalog is a setting, saved with Save.
  let { draft = $bindable() }: { draft: Settings } = $props();

  let tab = $state<"installed" | "catalog">("installed");
  // null: not loaded. The catalog is only loaded when the user presses the button, never on its own.
  let catalog = $state.raw<CatalogItem[] | null>(null);
  let catalogProblems = $state.raw<string[]>([]);
  let loadingCatalog = $state(false);
  let installing = $state<string | null>(null);

  let busy = $state(false);
  let problem = $state<string | null>(null);
  // The plugin waiting for the user to confirm what it may do, and the one waiting for a second click to go.
  let asking = $state<string | null>(null);
  let removing = $state<string | null>(null);

  onMount(() => {
    void pluginStore.load().catch((error) => (problem = String(error)));
  });

  // A button that is busy stays focusable (`aria-disabled`) so the keyboard is not thrown out of the dialog when
  // it greys out; the handlers ignore a press while busy.
  async function run(action: () => Promise<void>) {
    if (busy) return;
    busy = true;
    problem = null;
    try {
      await action();
      await pluginStore.load();
    } catch (error) {
      problem = String(error);
    } finally {
      busy = false;
    }
  }

  const install = () =>
    run(async () => {
      const plugin = await api.installPluginFile();
      if (plugin) app.toast("ok", `Installed “${plugin.manifest.name}”. It stays off until you turn it on.`);
    });

  /** The switch of the plugin row that holds `inside`, so the focus can be handed back to it. */
  const switchOf = (inside: EventTarget | null) =>
    (inside as HTMLElement | null)?.closest("li")?.querySelector<HTMLButtonElement>("[role=switch]") ?? null;

  // The confirmation closes with the button that was pressed in it, so the focus goes back to the switch.
  const confirmOn = async (plugin: Plugin, toggle: HTMLButtonElement | null) => {
    await run(async () => {
      await api.enablePlugin(plugin.manifest.id);
      asking = null;
      app.toast("ok", `“${plugin.manifest.name}” is on`);
    });
    toggle?.focus();
  };

  const turnOff = (plugin: Plugin) =>
    run(async () => {
      asking = null;
      await api.disablePlugin(plugin.manifest.id);
    });

  const uninstall = (plugin: Plugin) =>
    run(async () => {
      await api.uninstallPlugin(plugin.manifest.id);
      removing = null;
      app.toast("ok", `Uninstalled “${plugin.manifest.name}”`);
    });

  async function loadCatalog() {
    if (loadingCatalog || !app.settings.pluginsCatalog) return;
    loadingCatalog = true;
    problem = null;
    try {
      const view = await api.loadCatalog();
      catalog = view.items;
      catalogProblems = view.problems;
    } catch (error) {
      problem = String(error);
    } finally {
      loadingCatalog = false;
    }
  }

  async function installFromCatalog(item: CatalogItem) {
    if (installing !== null) return;
    installing = item.id;
    problem = null;
    try {
      const plugin = await api.installFromCatalog(item.id);
      // What is installed now, without asking the network again.
      catalog =
        catalog?.map((entry) =>
          entry.id === item.id
            ? { ...entry, installedVersion: plugin.manifest.version, updateAvailable: false }
            : entry,
        ) ?? null;
      await pluginStore.load();
      app.toast("ok", `Installed “${item.name}”, checked against the catalog. It stays off until you turn it on.`);
    } catch (error) {
      problem = String(error);
    } finally {
      installing = null;
    }
  }

  const confirmRemove =
    "rounded-md border border-err/50 bg-err/10 px-2 py-1 text-[10.5px] text-err transition-colors aria-disabled:opacity-40";

  /** Takes back "Uninstall for good?" and hands the focus to the button that asked it, which stays in place. */
  function keep(event: MouseEvent) {
    const row = (event.currentTarget as HTMLElement).parentElement;
    removing = null;
    row?.querySelector<HTMLButtonElement>("[data-remove]")?.focus();
  }

  const small =
    "rounded-md border border-line px-2 py-1 text-[10.5px] text-muted transition-colors hover:bg-hover hover:text-fg disabled:opacity-40 aria-disabled:opacity-40";
</script>

<div class="space-y-4">
  <div class="flex items-start justify-between gap-3">
    <p class="text-[10.5px] leading-relaxed text-faint">
      A plugin is a small WebAssembly program that runs in a sandbox: it cannot read your files, use the network or
      start programs. It can only do what it asks permission for, and only after you turn it on. Changes here apply
      at once.
    </p>
    <button type="button" class="{small} shrink-0" aria-disabled={busy} onclick={() => void install()}>
      Install from file…
    </button>
  </div>

  <div
    class="flex gap-1 rounded-lg border border-line bg-panel-2 p-1"
    role="group"
    aria-label="Plugins"
  >
    {#each [["installed", "Installed"], ["catalog", "Catalog"]] as const as [id, label] (id)}
      <button
        type="button"
        class="flex-1 rounded-md px-3 py-1.5 text-[12px] transition-colors
          {tab === id ? 'bg-elevated font-medium text-fg' : 'text-muted'}"
        aria-pressed={tab === id}
        onclick={() => (tab = id)}
      >
        {label}
      </button>
    {/each}
  </div>

  {#if tab === "catalog"}
    <section class="space-y-3">
      <p class="text-[10.5px] leading-relaxed text-faint">
        The catalog is a list of plugins published for TermZip. It is signed: TermZip checks the signature with a key
        built into the app, and the checksum of each plugin, before it trusts anything in it. It contacts the network
        only when you press <span class="text-muted">Load catalog</span>, never on its own.
      </p>
      <label class="flex items-start gap-2.5 text-[12px]">
        <input type="checkbox" bind:checked={draft.pluginsCatalog} class="mt-0.5 size-3.5 accent-accent" />
        <span>
          <span class="font-medium text-fg">Allow loading the plugin catalog</span>
          <span class="mt-0.5 block text-[10.5px] text-faint">Applies when you press Save. With this off, TermZip never contacts it.</span>
        </span>
      </label>
      <div class="flex items-center gap-2">
        <button
          type="button"
          class={small}
          aria-disabled={loadingCatalog || !app.settings.pluginsCatalog}
          onclick={() => void loadCatalog()}
        >
          {loadingCatalog ? "Loading…" : catalog ? "Reload the catalog" : "Load catalog"}
        </button>
        {#if !app.settings.pluginsCatalog}
          <span class="text-[10.5px] text-faint">Turned off. Allow it above and press Save.</span>
        {/if}
      </div>

      {#if catalog}
        {#if catalog.length === 0}
          <p class="rounded-lg border border-dashed border-line px-3 py-4 text-center text-[12px] text-faint">
            The catalog has no plugins yet.
          </p>
        {/if}
        <ul class="space-y-2.5">
          {#each catalog as item (item.id)}
            {@const action = catalogAction(item)}
            <li class="rounded-lg border border-line bg-panel-2 p-3">
              <div class="flex items-start gap-3">
                <div class="min-w-0 flex-1">
                  <div class="flex flex-wrap items-center gap-x-2 gap-y-1">
                    <span class="text-[12.5px] font-medium text-fg">{item.name}</span>
                    <span class="mono text-[10.5px] text-faint">{item.version}</span>
                    <span class="rounded bg-ok/10 px-1.5 py-0.5 text-[10px] font-medium text-ok">signed</span>
                  </div>
                  {#if item.description}
                    <p class="mt-1 text-[11.5px] text-muted">{item.description}</p>
                  {/if}
                  <p class="mt-1 text-[10.5px] text-faint">
                    {[item.author, item.license, formatSize(item.size)].filter(Boolean).join(" · ")}
                  </p>
                </div>
                <button
                  type="button"
                  class="shrink-0 rounded-md px-2.5 py-1 text-[11px] font-semibold transition-opacity disabled:opacity-40 aria-disabled:opacity-40
                    {action.enabled ? 'bg-accent text-white hover:opacity-90' : 'border border-line text-muted'}"
                  aria-disabled={!action.enabled || installing !== null}
                  onclick={() => {
                    if (action.enabled) void installFromCatalog(item);
                  }}
                >
                  {installing === item.id ? "Installing…" : action.label}
                </button>
              </div>
              {#if item.permissions.length > 0}
                <ul class="mt-2 flex flex-wrap gap-1.5" aria-label="Permissions">
                  {#each permissionChipsOf(item.permissions) as chip (chip.permission)}
                    <li
                      class="rounded-md border px-1.5 py-0.5 text-[10px] {chip.sensitive
                        ? 'border-warn/40 text-warn'
                        : 'border-line text-muted'}"
                    >
                      {chip.label}
                    </li>
                  {/each}
                </ul>
              {:else}
                <p class="mt-2 text-[10.5px] text-faint">Asks for no permission.</p>
              {/if}
            </li>
          {/each}
        </ul>
        {#if catalogProblems.length > 0}
          <div class="rounded-lg border border-warn/40 bg-warn/10 px-3 py-2 text-[11px] text-warn">
            <p class="mb-1 font-medium">Some entries of the catalog could not be used:</p>
            <ul class="list-disc space-y-0.5 pl-4">
              {#each catalogProblems as sentence (sentence)}
                <li>{sentence}</li>
              {/each}
            </ul>
          </div>
        {/if}
      {/if}
    </section>
  {/if}

  {#if tab === "installed"}
  <section>
    <h3 class={heading}>Installed</h3>
    {#if pluginStore.plugins.length === 0}
      <p class="rounded-lg border border-dashed border-line px-3 py-4 text-center text-[12px] text-faint">
        {pluginStore.loaded ? "No plugins are installed." : "Looking for plugins…"}
      </p>
    {:else}
      <ul class="space-y-2.5">
        {#each pluginStore.plugins as plugin (plugin.manifest.id)}
          {@const state = stateOf(plugin)}
          {@const source = describeSource(plugin)}
          <li class="rounded-lg border bg-panel-2 p-3 {state === 'running' ? 'border-accent/60' : 'border-line'}">
            <div class="flex items-start gap-3">
              <div class="min-w-0 flex-1">
                <div class="flex flex-wrap items-center gap-x-2 gap-y-1">
                  <span class="text-[12.5px] font-medium text-fg">{plugin.manifest.name}</span>
                  <span class="mono text-[10.5px] text-faint">{plugin.manifest.version}</span>
                  <span
                    class="rounded px-1.5 py-0.5 text-[10px] font-medium {source.verified
                      ? 'bg-ok/10 text-ok'
                      : 'bg-warn/10 text-warn'}"
                    title={source.label}
                  >
                    {source.verified ? "verified" : "not verified"}
                  </span>
                </div>
                {#if plugin.manifest.description}
                  <p class="mt-1 text-[11.5px] text-muted">{plugin.manifest.description}</p>
                {/if}
                <p class="mt-1 text-[10.5px] text-faint">
                  {[plugin.manifest.author, plugin.manifest.license].filter(Boolean).join(" · ")}
                  {#if plugin.manifest.author || plugin.manifest.license}·{/if}
                  {describeAdds(plugin.manifest)}
                </p>
              </div>
              <button
                type="button"
                role="switch"
                aria-checked={plugin.enabled}
                aria-label="Turn {plugin.manifest.name} {plugin.enabled ? 'off' : 'on'}"
                class="relative h-5 w-9 shrink-0 rounded-full border transition-colors disabled:opacity-40 aria-disabled:opacity-40
                  {plugin.enabled ? 'border-accent bg-accent' : 'border-line-strong bg-panel'}"
                aria-disabled={busy}
                onclick={() => (plugin.enabled ? void turnOff(plugin) : (asking = plugin.manifest.id))}
              >
                <span
                  class="absolute top-0.5 size-3.5 rounded-full bg-white shadow transition-all
                    {plugin.enabled ? 'left-[18px]' : 'left-0.5'}"
                ></span>
              </button>
            </div>

            {#if plugin.manifest.permissions.length > 0}
              <ul class="mt-2 flex flex-wrap gap-1.5" aria-label="Permissions">
                {#each permissionChips(plugin.manifest) as chip (chip.permission)}
                  <li
                    class="rounded-md border px-1.5 py-0.5 text-[10px] {chip.sensitive
                      ? 'border-warn/40 text-warn'
                      : 'border-line text-muted'}"
                  >
                    {chip.label}
                  </li>
                {/each}
              </ul>
            {/if}

            {#if state === "stopped" || state === "changed"}
              <p class="mt-2 text-[10.5px] text-warn" role="status">{describeState(state)}</p>
            {/if}

            {#if asking === plugin.manifest.id}
              <div class="mt-3 rounded-lg border border-accent/50 bg-panel p-3" role="group" aria-label="Confirm">
                <p class="text-[11.5px] font-medium text-fg">{describeAsk(plugin)}</p>
                {#if plugin.permissionNotes.length > 0}
                  <ul class="mt-1.5 list-disc space-y-0.5 pl-4 text-[11px] text-muted">
                    {#each plugin.permissionNotes as note (note.permission)}
                      <li>{note.text}</li>
                    {/each}
                  </ul>
                {/if}
                {#if !source.verified}
                  <p class="mt-2 text-[10.5px] text-warn">
                    This file did not come from the catalog and nobody has verified it. Turn it on only if you trust
                    where it came from.
                  </p>
                {/if}
                <div class="mt-2.5 flex justify-end gap-2">
                  <button
                    type="button"
                    class={small}
                    onclick={(event) => {
                      const toggle = switchOf(event.currentTarget);
                      asking = null;
                      toggle?.focus();
                    }}
                  >
                    Cancel
                  </button>
                  <button
                    type="button"
                    class="rounded-md bg-accent px-2.5 py-1 text-[11px] font-semibold text-white hover:opacity-90 disabled:opacity-40 aria-disabled:opacity-40"
                    aria-disabled={busy}
                    onclick={(event) => void confirmOn(plugin, switchOf(event.currentTarget))}
                  >
                    Turn on
                  </button>
                </div>
              </div>
            {/if}

            <div class="mt-2.5 flex items-center gap-2">
              <!-- One button for both steps, so the focus stays on it when it asks "for good?". -->
              <button
                type="button"
                data-remove
                class={removing === plugin.manifest.id ? confirmRemove : small}
                aria-disabled={busy}
                onclick={() => {
                  if (removing === plugin.manifest.id) void uninstall(plugin);
                  else removing = plugin.manifest.id;
                }}
              >
                {removing === plugin.manifest.id ? "Uninstall for good?" : "Uninstall"}
              </button>
              {#if removing === plugin.manifest.id}
                <button type="button" class={small} onclick={keep}>Keep</button>
              {/if}
              {#if plugin.manifest.homepage}
                <span class="truncate text-[10.5px] text-faint" title={plugin.manifest.homepage}>
                  {plugin.manifest.homepage}
                </span>
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
  {/if}

  {#if problem}
    <p class="rounded-lg border border-err/40 bg-err/10 px-3 py-2 text-[11px] text-err" role="alert">{problem}</p>
  {/if}

  {#if tab === "installed" && pluginStore.problems.length > 0}
    <div class="rounded-lg border border-warn/40 bg-warn/10 px-3 py-2 text-[11px] text-warn">
      <p class="mb-1 font-medium">Some plugin files were left out or could not start:</p>
      <ul class="list-disc space-y-0.5 pl-4">
        {#each pluginStore.problems as sentence (sentence)}
          <li>{sentence}</li>
        {/each}
      </ul>
    </div>
  {/if}
</div>
