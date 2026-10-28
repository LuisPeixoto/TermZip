<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "./lib/api";
  import { app } from "./lib/stores/app.svelte";
  import { persistWorkspace, type Persistence } from "./lib/stores/persist.svelte";
  import { workspace } from "./lib/stores/workspace.svelte";
  import { matchShortcut, tabPosition, type AppShortcut } from "./lib/shortcuts";
  import { focusTerminal } from "./lib/focus";
  import Toolbar from "./lib/components/Toolbar.svelte";
  import PaneGrid from "./lib/components/PaneGrid.svelte";
  import BlocksRail from "./lib/components/BlocksRail.svelte";
  import StatusBar from "./lib/components/StatusBar.svelte";
  import SettingsModal from "./lib/components/SettingsModal.svelte";
  import Toasts from "./lib/components/Toasts.svelte";

  onMount(() => {
    let persistence: Persistence | undefined;

    void (async () => {
      try {
        await app.init();
      } catch (error) {
        app.toast("err", `Could not load preferences: ${error}`);
      }
      // The saved layout goes in before anything is shown, or a shell would start for the fresh tab and be thrown away.
      try {
        workspace.restore(await api.loadWorkspace());
      } catch (error) {
        app.toast("warn", `Could not restore the last session: ${error}`);
        workspace.restore(null);
      }
      persistence = persistWorkspace(workspace, api.saveWorkspace);
    })();

    // A window that is closing cannot wait for the debounce.
    const flush = () => persistence?.flush();
    window.addEventListener("beforeunload", flush);
    return () => {
      window.removeEventListener("beforeunload", flush);
      persistence?.stop();
    };
  });

  function runShortcut(shortcut: AppShortcut) {
    const position = tabPosition(shortcut);
    if (position !== null) {
      workspace.activateAt(position);
      focusTerminal();
      return;
    }
    switch (shortcut) {
      case "restart":
        window.dispatchEvent(new CustomEvent("termzip:restart"));
        break;
      case "blocks":
        app.toggleBlocks();
        break;
      case "find":
        workspace.activePane.openSearch();
        break;
      case "newTab":
        workspace.newTab();
        focusTerminal();
        break;
      case "closePane":
        workspace.closeActivePane();
        focusTerminal();
        break;
      case "splitRight":
      case "splitDown":
        if (!workspace.splitActive(shortcut === "splitRight" ? "row" : "column")) {
          app.toast("warn", "This tab cannot hold another pane");
        }
        focusTerminal();
        break;
      case "focusLeft":
      case "focusRight":
      case "focusUp":
      case "focusDown":
        workspace.activeTab.focusSide(shortcut.slice(5).toLowerCase() as "left" | "right" | "up" | "down");
        focusTerminal();
        break;
      case "nextTab":
      case "prevTab":
        workspace.cycle(shortcut === "nextTab" ? 1 : -1);
        focusTerminal();
        break;
    }
  }

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
      runShortcut(shortcut);
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
  {#if app.ready && workspace.loaded}
    <div class="flex min-h-0 flex-1">
      <!-- Every tab stays mounted, so its terminal keeps its buffer; only the active one is shown. -->
      <div class="relative min-h-0 min-w-0 flex-1">
        {#each workspace.tabs as tab (tab.id)}
          <div class="absolute inset-0 flex flex-col" class:hidden={tab.id !== workspace.activeId}>
            <PaneGrid {tab} />
          </div>
        {/each}
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