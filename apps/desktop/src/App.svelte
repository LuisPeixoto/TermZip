<script lang="ts">
  import { onMount } from "svelte";
  import { onBlocksUpdate } from "./lib/api";
  import { app } from "./lib/stores/app.svelte";
  import Toolbar from "./lib/components/Toolbar.svelte";
  import TerminalPane from "./lib/components/TerminalPane.svelte";
  import BlocksRail from "./lib/components/BlocksRail.svelte";
  import Composer from "./lib/components/Composer.svelte";
  import StatusBar from "./lib/components/StatusBar.svelte";
  import SettingsModal from "./lib/components/SettingsModal.svelte";
  import Toasts from "./lib/components/Toasts.svelte";

  onMount(() => {
    void app
      .init()
      .catch((error) => app.toast("err", `Could not load preferences: ${error}`));
    let unlisten: (() => void) | undefined;
    void onBlocksUpdate((event) => {
      const session = app.session;
      if (session && event.sessionId === session.id) {
        app.setBlocks(event.blocks);
      }
    }).then((unlistenFn) => (unlisten = unlistenFn));
    return () => unlisten?.();
  });

  function onWindowKeydown(event: KeyboardEvent) {
    const mod = event.metaKey || event.ctrlKey;
    const typing =
      event.target instanceof HTMLInputElement ||
      event.target instanceof HTMLTextAreaElement ||
      (event.target instanceof HTMLElement && event.target.classList.contains("xterm"));

    if (mod && event.key.toLowerCase() === "," && !event.shiftKey) {
      event.preventDefault();
      app.toggleSettings();
    } else if (mod && event.key.toLowerCase() === "r" && !typing) {
      event.preventDefault();
      window.dispatchEvent(new CustomEvent("termzip:restart"));
    } else if (mod && event.key.toLowerCase() === "k" && !typing) {
      event.preventDefault();
      window.dispatchEvent(
        new CustomEvent("termzip:composer", { detail: { command: "", execute: false } }),
      );
    } else if (mod && event.key.toLowerCase() === "b" && !typing) {
      event.preventDefault();
      app.toggleBlocks();
    } else if (event.key === "Escape" && typing) {
      (event.target as HTMLElement).blur();
    }
  }
</script>

<svelte:window onkeydown={onWindowKeydown} oncontextmenu={(event) => event.preventDefault()} />

<div class="flex h-full flex-col bg-bg">
  <Toolbar />
  {#if app.ready}
    <div class="flex min-h-0 flex-1">
      <div class="flex min-w-0 flex-1 flex-col">
        <TerminalPane />
        <Composer />
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