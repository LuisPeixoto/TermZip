<script lang="ts">
  import Icon from "./Icon.svelte";
  import { app } from "../stores/app.svelte";
  import { workspace } from "../stores/workspace.svelte";
  import { focusTerminal } from "../focus";

  let strip: HTMLDivElement | undefined = $state();
  let renaming = $state<string | null>(null);
  let draft = $state("");
  let dragging = $state<string | null>(null);
  // A drag ends with a click on the tab it ended on; that click must not switch tabs.
  let suppressClick = false;
  let menuOpen = $state(false);

  function openWithProfile(profileId: string) {
    menuOpen = false;
    if (!workspace.newTab(profileId)) app.toast("warn", "That is the most tabs a window can hold");
    focusTerminal();
  }

  function activate(id: string, clicks: number) {
    if (suppressClick) {
      suppressClick = false;
      return;
    }
    workspace.activate(id);
    // The second click of a double click is about to open the rename field; taking the keyboard
    // away from it would close the field at once.
    if (clicks < 2) focusTerminal();
  }

  function close(id: string) {
    workspace.closeTab(id);
    focusTerminal();
  }

  function startRename(id: string, current: string) {
    renaming = id;
    draft = current;
  }

  function finishRename(save: boolean) {
    if (renaming && save) workspace.renameTab(renaming, draft);
    renaming = null;
    focusTerminal();
  }

  function focusOnMount(node: HTMLInputElement) {
    node.focus();
    // The bound value is written right after the action runs; select it once it is there.
    queueMicrotask(() => node.select());
  }

  // Reordering uses pointer events on the window: moving a keyed element in the DOM would drop a
  // pointer capture, and HTML5 drag and drop is swallowed by the webview's file-drop handler.
  function positionAt(x: number): number {
    const tabs = [...(strip?.querySelectorAll<HTMLElement>("[data-tab]") ?? [])];
    for (let index = 0; index < tabs.length; index += 1) {
      const box = tabs[index].getBoundingClientRect();
      if (x < box.left + box.width / 2) return index;
    }
    return Math.max(0, tabs.length - 1);
  }

  function onPointerDown(event: PointerEvent, id: string) {
    if (event.button !== 0 || renaming) return;
    const startX = event.clientX;
    let moved = false;

    const onMove = (move: PointerEvent) => {
      if (!moved && Math.abs(move.clientX - startX) < 5) return;
      moved = true;
      dragging = id;
      workspace.moveTab(id, positionAt(move.clientX));
    };
    const onUp = () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onUp);
      if (moved) {
        // The drag ends with a click on the tab under the pointer, which must not switch tabs. If
        // that click never comes (the pointer left the tab), do not let it linger for the next one.
        suppressClick = true;
        setTimeout(() => (suppressClick = false), 80);
        focusTerminal(); // pressing the tab moved the keyboard onto its button
      }
      dragging = null;
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onUp);
  }
</script>

<svelte:window
  onpointerdown={(event) => {
    if (menuOpen && !(event.target as Element | null)?.closest("[data-profile-menu]")) menuOpen = false;
  }}
/>

<div bind:this={strip} class="flex min-w-0 items-center gap-1" role="tablist" aria-label="Tabs">
  {#each workspace.tabs as tab (tab.id)}
    {@const active = tab.id === workspace.activeId}
    <div
      data-tab
      class="group relative flex h-7 min-w-[84px] max-w-[200px] shrink items-center rounded-md border text-[11.5px] transition-colors
        {active ? 'border-line bg-panel-2 text-fg' : 'border-transparent text-muted hover:bg-hover hover:text-fg'}
        {dragging === tab.id ? 'opacity-70' : ''}"
      role="presentation"
    >
      {#if renaming === tab.id}
        <input
          use:focusOnMount
          bind:value={draft}
          maxlength="64"
          aria-label="Tab name"
          class="mx-1 h-5 min-w-0 flex-1 rounded border border-accent bg-panel px-1.5 text-[11.5px] text-fg outline-none"
          onkeydown={(event) => {
            if (event.key === "Enter") finishRename(true);
            else if (event.key === "Escape") finishRename(false);
            event.stopPropagation();
          }}
          onblur={() => renaming === tab.id && finishRename(true)}
        />
      {:else}
        <button
          type="button"
          role="tab"
          aria-selected={active}
          class="flex min-w-0 flex-1 items-center gap-1.5 truncate px-2 text-left"
          title={tab.title}
          onpointerdown={(event) => onPointerDown(event, tab.id)}
          onclick={(event) => activate(tab.id, event.detail)}
          ondblclick={() => startRename(tab.id, tab.title)}
        >
          {#if tab.active.exited}
            <span class="size-1.5 shrink-0 rounded-full bg-warn" title="Shell exited"></span>
          {/if}
          <span class="truncate">{tab.title}</span>
        </button>
        <button
          type="button"
          class="mr-1 grid size-4 shrink-0 place-items-center rounded text-faint transition-opacity hover:bg-hover hover:text-fg
            {active ? 'opacity-100' : 'opacity-0 group-hover:opacity-100 focus-visible:opacity-100'}"
          aria-label="Close tab {tab.title}"
          onclick={() => close(tab.id)}
        >
          <Icon name="x" size={11} stroke={2.2} />
        </button>
      {/if}
    </div>
  {/each}

  <button
    type="button"
    class="grid size-7 shrink-0 place-items-center rounded-md text-muted transition-colors hover:bg-hover hover:text-fg"
    title="New tab{app.hint('newTab')}"
    aria-label="New tab"
    onclick={() => {
      if (!workspace.newTab()) app.toast("warn", "That is the most tabs a window can hold");
      focusTerminal();
    }}
  >
    <Icon name="plus" size={14} />
  </button>

  {#if app.settings.profiles.length > 0}
    <div class="relative shrink-0" data-profile-menu>
      <button
        type="button"
        class="grid size-7 place-items-center rounded-md text-muted transition-colors hover:bg-hover hover:text-fg"
        title="New tab with a profile"
        aria-label="New tab with a profile"
        aria-haspopup="menu"
        aria-expanded={menuOpen}
        onclick={() => (menuOpen = !menuOpen)}
      >
        <Icon name="chevronRight" size={11} stroke={2.2} class="rotate-90" />
      </button>
      {#if menuOpen}
        <div
          role="menu"
          tabindex="-1"
          class="shadow-pop absolute left-0 top-8 z-30 min-w-[190px] rounded-lg border border-line bg-elevated p-1"
          onkeydown={(event) => event.key === "Escape" && (menuOpen = false)}
        >
          <button
            type="button"
            role="menuitem"
            class="flex w-full items-center rounded-md px-2.5 py-1.5 text-left text-[11.5px] text-fg hover:bg-hover"
            onclick={() => openWithProfile("default")}
          >
            Built-in
          </button>
          {#each app.settings.profiles as profile (profile.id)}
            <button
              type="button"
              role="menuitem"
              class="flex w-full items-center rounded-md px-2.5 py-1.5 text-left text-[11.5px] text-fg hover:bg-hover"
              onclick={() => openWithProfile(profile.id)}
            >
              <span class="truncate">{profile.name}</span>
              {#if app.settings.defaultProfile === profile.id}
                <span class="ml-auto pl-2 text-[10px] text-accent">default</span>
              {/if}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>
