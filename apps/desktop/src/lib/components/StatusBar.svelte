<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./Icon.svelte";
  import { describeGit } from "../git";
  import { profileFor } from "../paneLook";
  import { app } from "../stores/app.svelte";
  import { git } from "../stores/git.svelte";
  import { workspace } from "../stores/workspace.svelte";
  import { pluginStore } from "../stores/plugins.svelte";

  const items = $derived(Object.entries(pluginStore.statuses));

  // The git state follows the folder of the active terminal. Only strings and a number are read here, not the
  // pane, so the effect runs again when the folder or the commands change, and not on every poll.
  const pane = $derived(workspace.activePane);
  const folder = $derived(pane.cwd ?? pane.restoreDirectory ?? null);
  // The folder is a folder of this computer: it means nothing for an SSH connection or a WSL distribution.
  const local = $derived((profileFor(app.settings, pane.profileId)?.kind ?? "local") === "local");
  const watched = $derived(app.settings.gitStatus && local ? folder : null);
  const finishedCommands = $derived(pane.blocks.length);
  $effect(() => {
    finishedCommands;
    git.watch(watched);
  });
  onMount(() => {
    // A commit made in another window or app shows up when this one comes back to the front.
    const onFocus = () => git.refresh();
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  });
  const view = $derived(git.status && watched ? describeGit(git.status) : null);
</script>

<footer
  class="flex h-7 shrink-0 items-center gap-3 border-t border-line bg-window-panel px-3 text-[10.5px] text-muted"
>
  <span class="flex items-center gap-1.5">
    <span
      class="size-1.5 rounded-full {workspace.activePane.exited ? 'bg-warn' : 'bg-ok'}"
      title={workspace.activePane.exited ? "Shell exited" : "Shell running"}
    ></span>
    {#if workspace.activePane.exited}
      shell exited
    {:else if workspace.activePane.session?.integrated}
      shell integration
    {:else}
      plain terminal
    {/if}
  </span>

  <span class="text-line-strong">·</span>

  {#if workspace.activePane.session && !workspace.activePane.session.integrated}
    <span class="text-warn" title="This shell does not emit OSC 133 markers; blocks are unavailable">
      no block capture
    </span>
    <span class="text-line-strong">·</span>
  {/if}

  <span class="mono">
    {workspace.activePane.viewport.columns}×{workspace.activePane.viewport.rows}
  </span>

  {#if view}
    <span class="text-line-strong">·</span>
    <span class="mono flex items-center gap-1" title={view.title}>
      <Icon name="gitBranch" size={11} />
      <span class="max-w-[12rem] truncate">{view.label}</span>
      {#if view.sync}<span class="text-faint">{view.sync}</span>{/if}
      {#if view.conflicted}
        <span class="text-err" aria-label="Merge conflict">!</span>
      {:else if view.dirty}
        <span class="size-1.5 rounded-full bg-warn" aria-label="Uncommitted changes"></span>
      {/if}
    </span>
  {/if}

  {#each items as [plugin, text] (plugin)}
    <span class="text-line-strong">·</span>
    <span class="max-w-[16rem] truncate" title="From the plugin {pluginStore.name(plugin)}">{text}</span>
  {/each}

  <span class="flex-1"></span>

  <span class="hidden gap-3 text-faint md:flex" aria-label="Keyboard shortcuts">
    {#if app.shortcut("blocks")}<span><kbd class="mono">{app.shortcut("blocks")}</kbd> blocks</span>{/if}
    {#if app.shortcut("restart")}<span><kbd class="mono">{app.shortcut("restart")}</kbd> restart</span>{/if}
    {#if app.shortcut("preferences")}<span><kbd class="mono">{app.shortcut("preferences")}</kbd> preferences</span>{/if}
  </span>

  <span class="mono text-faint">
    TermZip {app.version}
  </span>
</footer>