<script lang="ts">
  import { app } from "../../stores/app.svelte";
  import { api, type Settings } from "../../api";
  import { failed, found, summary, type UpdateState } from "../../updates";
  import { field, heading, hint } from "./styles";

  let { draft = $bindable() }: { draft: Settings } = $props();

  // Updates are looked for only when this button is pressed (or at startup, if the checkbox below is on).
  let update = $state<UpdateState>({ kind: "idle" });

  const busy = $derived(update.kind === "checking" || update.kind === "installing");

  async function checkNow() {
    // Not a disabled button: pressing it would make the button lose the keyboard, and the next Tab would start over
    // from the top of the dialog.
    if (busy) return;
    update = { kind: "checking" };
    try {
      update = found(await api.checkForUpdate());
    } catch (error) {
      update = failed(error);
    }
  }

  async function install() {
    if (busy) return;
    update = { kind: "installing" };
    try {
      // The app restarts when this succeeds.
      await api.installUpdate();
    } catch (error) {
      update = failed(error);
    }
  }
</script>

<div class="space-y-5">
  <section>
    <label class="flex items-start gap-2.5 text-[12px]">
      <input
        type="checkbox"
        bind:checked={draft.restoreSession}
        class="mt-0.5 size-3.5 accent-accent"
      />
      <span>
        <span class="font-medium text-fg">Restore tabs and folders on launch</span>
        <span class="mt-0.5 block text-[10.5px] text-faint">
          Reopens your tabs, splits and the folder each shell was in. Terminal output is never saved.
        </span>
      </span>
    </label>
  </section>

  <section>
    <label class="flex items-start gap-2.5 text-[12px]">
      <input type="checkbox" bind:checked={draft.gitStatus} class="mt-0.5 size-3.5 accent-accent" />
      <span>
        <span class="font-medium text-fg">Use git in the status bar and the palette</span>
        <span class="mt-0.5 block text-[10.5px] text-faint">
          Shows the branch, how far it is from its upstream and whether files changed, and offers <em>Open the
          repository in the browser</em>, for the folder of the terminal. It runs <span class="mono">git</span> on
          your computer and never uses the network; with this off git is never started. SSH and WSL terminals do
          not use it.
        </span>
      </span>
    </label>
  </section>

  <section>
    <label for="pref-shell" class={heading}>Shell path</label>
    <input
      id="pref-shell"
      type="text"
      list="termzip-shell-paths"
      placeholder="System default shell"
      class={field}
      value={draft.shell ?? ""}
      oninput={(event) => {
        const raw = event.currentTarget.value.trim();
        draft = { ...draft, shell: raw || null };
      }}
    />
    <datalist id="termzip-shell-paths">
      {#each app.shells as shell (shell.path)}
        <option value={shell.path}>{shell.label}{shell.default ? " · default" : ""}</option>
      {/each}
    </datalist>
    <p class={hint}>Integrated shells: zsh, bash, fish, PowerShell. Other shells run without block capture.</p>
  </section>

  <section>
    <label for="pref-directory" class={heading}>Working directory</label>
    <input
      id="pref-directory"
      type="text"
      placeholder="System default"
      class={field}
      value={draft.workingDirectory ?? ""}
      oninput={(event) => {
        const raw = event.currentTarget.value.trim();
        draft = { ...draft, workingDirectory: raw || null };
      }}
    />
  </section>

  <section>
    <p class={heading}>Updates</p>
    <div class="flex flex-wrap items-center gap-2.5 text-[12px]">
      <span class="text-muted">TermZip <span class="mono text-fg">{app.version}</span></span>
      <button
        type="button"
        class="rounded-md border border-line px-2.5 py-1 text-[11px] text-fg hover:bg-hover aria-disabled:opacity-50"
        aria-disabled={busy}
        onclick={checkNow}
      >
        {update.kind === "checking" ? "Checking…" : "Check for updates"}
      </button>
      {#if update.kind === "current"}
        <span class="text-ok" role="status">You have the newest version.</span>
      {/if}
    </div>
    {#if update.kind === "available"}
      <div class="mt-2.5 space-y-2 rounded-lg border border-line bg-panel-2 p-3" role="status">
        <p class="text-[12px] text-fg">{summary(update.info)}</p>
        {#if update.info.notes}
          <pre class="mono max-h-40 overflow-auto whitespace-pre-wrap text-[11px] text-muted">{update.info.notes}</pre>
        {/if}
        <button
          type="button"
          class="rounded-md bg-accent px-2.5 py-1 text-[11px] font-semibold text-on-accent hover:opacity-90"
          onclick={install}
        >
          Install and restart
        </button>
      </div>
    {:else if update.kind === "installing"}
      <p class="mt-2.5 text-[11.5px] text-muted" role="status">Downloading and checking the update… TermZip restarts when it is installed.</p>
    {:else if update.kind === "error"}
      <p class="mt-2.5 text-[11.5px] text-err" role="alert">{update.message}</p>
    {/if}
    <label class="mt-3 flex items-start gap-2.5 text-[12px]">
      <input type="checkbox" bind:checked={draft.checkUpdatesOnLaunch} class="mt-0.5 size-3.5 accent-accent" />
      <span>
        <span class="font-medium text-fg">Check for updates when TermZip starts</span>
        <span class="mt-0.5 block text-[10.5px] text-faint">
          Off by default. It asks the update server (github.com) once at startup and only tells you if there is one: it
          never installs by itself, and every update is verified against the key built into TermZip before it is used.
        </span>
      </span>
    </label>
  </section>
</div>
