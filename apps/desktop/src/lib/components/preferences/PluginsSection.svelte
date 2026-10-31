<script lang="ts">
  import { onMount } from "svelte";
  import { api, type Plugin, type Settings } from "../../api";
  import { app } from "../../stores/app.svelte";
  import { pluginStore } from "../../stores/plugins.svelte";
  import {
    describeAdds,
    describeAsk,
    describeSource,
    describeState,
    permissionChips,
    stateOf,
  } from "../../plugins";
  import { heading } from "./styles";

  // Every section is handed the dialog's draft; plugins apply at once and have no use for it.
  let { draft: _draft }: { draft?: Settings } = $props();

  let busy = $state(false);
  let problem = $state<string | null>(null);
  // The plugin waiting for the user to confirm what it may do, and the one waiting for a second click to go.
  let asking = $state<string | null>(null);
  let removing = $state<string | null>(null);

  onMount(() => {
    void pluginStore.load().catch((error) => (problem = String(error)));
  });

  async function run(action: () => Promise<void>) {
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

  const confirmOn = (plugin: Plugin) =>
    run(async () => {
      await api.enablePlugin(plugin.manifest.id);
      asking = null;
      app.toast("ok", `“${plugin.manifest.name}” is on`);
    });

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

  const small =
    "rounded-md border border-line px-2 py-1 text-[10.5px] text-muted transition-colors hover:bg-hover hover:text-fg disabled:opacity-40";
</script>

<div class="space-y-4">
  <div class="flex items-start justify-between gap-3">
    <p class="text-[10.5px] leading-relaxed text-faint">
      A plugin is a small WebAssembly program that runs in a sandbox: it cannot read your files, use the network or
      start programs. It can only do what it asks permission for, and only after you turn it on. Changes here apply
      at once.
    </p>
    <button type="button" class="{small} shrink-0" disabled={busy} onclick={() => void install()}>
      Install from file…
    </button>
  </div>

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
                class="relative h-5 w-9 shrink-0 rounded-full border transition-colors disabled:opacity-40
                  {plugin.enabled ? 'border-accent bg-accent' : 'border-line-strong bg-panel'}"
                disabled={busy}
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
                  <button type="button" class={small} onclick={() => (asking = null)}>Cancel</button>
                  <button
                    type="button"
                    class="rounded-md bg-accent px-2.5 py-1 text-[11px] font-semibold text-white hover:opacity-90 disabled:opacity-40"
                    disabled={busy}
                    onclick={() => void confirmOn(plugin)}
                  >
                    Turn on
                  </button>
                </div>
              </div>
            {/if}

            <div class="mt-2.5 flex items-center gap-2">
              {#if removing === plugin.manifest.id}
                <button
                  type="button"
                  class="rounded-md border border-err/50 bg-err/10 px-2 py-1 text-[10.5px] text-err disabled:opacity-40"
                  disabled={busy}
                  onclick={() => void uninstall(plugin)}
                >
                  Uninstall for good?
                </button>
                <button type="button" class={small} onclick={() => (removing = null)}>Keep</button>
              {:else}
                <button type="button" class={small} disabled={busy} onclick={() => (removing = plugin.manifest.id)}>
                  Uninstall
                </button>
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

  {#if problem}
    <p class="rounded-lg border border-err/40 bg-err/10 px-3 py-2 text-[11px] text-err" role="alert">{problem}</p>
  {/if}

  {#if pluginStore.problems.length > 0}
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
