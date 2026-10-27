<script lang="ts">
  import { onMount } from "svelte";
  import { app } from "./lib/stores/app.svelte";
  import { matchShortcut } from "./lib/shortcuts";
  import Toolbar from "./lib/components/Toolbar.svelte";
  import TerminalPane from "./lib/components/TerminalPane.svelte";
  import BlocksRail from "./lib/components/BlocksRail.svelte";
  import StatusBar from "./lib/components/StatusBar.svelte";
  import SettingsModal from "./lib/components/SettingsModal.svelte";
  import Toasts from "./lib/components/Toasts.svelte";

  onMount(() => {
    void app
      .init()
      .catch((error) => app.toast("err", `Could not load preferences: ${error}`));
  });

  function onWindowKeydown(event: KeyboardEvent) {
    // xterm keeps focus in a hidden textarea of its own; that is the terminal, not a form field.
    const target = event.target instanceof Element ? event.target : null;
    const inField =
      target !== null &&
      target.closest(".xterm") === null &&
      (target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        target instanceof HTMLSelectElement);

    const shortcut = matchShortcut(event, app.platform);
    if (shortcut === "preferences") {
      event.preventDefault();
      app.toggleSettings();
    } else if (shortcut && !inField && !app.settingsOpen) {
      event.preventDefault();
      if (shortcut === "restart") window.dispatchEvent(new CustomEvent("termzip:restart"));
      else app.toggleBlocks();
    } else if (event.key === "Escape") {
      if (app.settingsOpen) {
        event.preventDefault();
        app.closeSettings();
      } else if (inField) {
        (target as HTMLElement).blur();
      }
    }
  }
</script>

<svelte:window onkeydown={onWindowKeydown} oncontextmenu={(event) => event.preventDefault()} />

<div class="flex h-full flex-col bg-bg" inert={app.settingsOpen}>
  <Toolbar />
  {#if app.ready}
    <div class="flex min-h-0 flex-1">
      <div class="flex min-w-0 flex-1 flex-col">
        <TerminalPane />
      </div>
      {#if app.blocksOpen}
        <BlocksRail />
      {/if}
    </div>
    <StatusBar />
  {:else}
    <div class="grid min-h-0 flex-1 place-items-center text-[12px] text-faint">
      Starting TermZip…
    </div>
  {/if}
</div>

{#if app.settingsOpen}
  <SettingsModal />
{/if}
<Toasts />