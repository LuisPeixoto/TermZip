<script lang="ts">
  import Icon from "./Icon.svelte";
  import { api, type CursorStyle, type Profile, type ProfileKind } from "../api";
  import { app } from "../stores/app.svelte";
  import {
    MAX_PROFILES,
    describeProfile,
    formatEnv,
    formatFolders,
    parseEnv,
    parseFolders,
    parseFontSize,
    parsePort,
    uniqueProfileId,
  } from "../profiles";
  import { themes } from "../stores/themes.svelte";
  import { groupThemes } from "../theme";

  let {
    profiles = $bindable(),
    defaultProfile = $bindable(),
  }: { profiles: Profile[]; defaultProfile: string | null } = $props();

  // `null` is the list, `""` a new profile, anything else the id being edited.
  let editing = $state<string | null>(null);
  const blankForm = { name: "", kind: "local" as ProfileKind, host: "", user: "", port: "", distribution: "", shell: "", workingDirectory: "", folders: "", theme: "", fontSize: "", fontFamily: "", cursorStyle: "", env: "" };
  let form = $state({ ...blankForm });
  const groups = $derived.by(() => {
    const { dark, light } = groupThemes(themes.entries);
    return [
      { label: "Dark themes", entries: dark },
      { label: "Light themes", entries: light },
    ];
  });
  // Ids to names, for the list; a theme that is gone is shown by its id.
  const themeNames = $derived(new Map(themes.entries.map((entry) => [entry.theme.id, entry.theme.name])));
  let problem = $state<string | null>(null);
  // What the host and distribution fields suggest, read once the first time a form needs them.
  let sshHosts = $state<string[]>([]);
  let distributions = $state<string[]>([]);
  let asked = { ssh: false, wsl: false };
  $effect(() => {
    if (editing === null) return;
    if (form.kind === "ssh" && !asked.ssh) {
      asked.ssh = true;
      api.listSshHosts().then((hosts) => (sshHosts = hosts)).catch(() => {});
    } else if (form.kind === "wsl" && !asked.wsl) {
      asked.wsl = true;
      api.listWslDistributions().then((list) => (distributions = list)).catch(() => {});
    }
  });
  // WSL exists on Windows; a profile that is already WSL (a config brought from there) stays editable anywhere.
  const wslOffered = $derived(app.platform === "windows" || form.kind === "wsl");

  const field =
    "mono w-full rounded-lg border border-line bg-panel-2 px-2.5 py-1.5 text-[11.5px] outline-none focus:border-accent";
  const small = "mb-1 block text-[10.5px] font-medium text-muted";

  // The form opens at the bottom of a scrolling dialog: bring it into view and start typing.
  function focusFirst(node: HTMLInputElement) {
    node.focus();
    node.scrollIntoView({ block: "nearest" });
  }

  function startNew() {
    form = { ...blankForm };
    problem = null;
    editing = "";
  }

  function startEdit(profile: Profile) {
    form = {
      name: profile.name,
      kind: profile.kind ?? "local",
      host: profile.host ?? "",
      user: profile.user ?? "",
      port: profile.port ? String(profile.port) : "",
      distribution: profile.distribution ?? "",
      shell: profile.shell ?? "",
      workingDirectory: profile.workingDirectory ?? "",
      folders: formatFolders(profile.folders),
      theme: profile.theme ?? "",
      fontSize: profile.fontSize ? String(profile.fontSize) : "",
      fontFamily: profile.fontFamily ?? "",
      cursorStyle: profile.cursorStyle ?? "",
      env: formatEnv(profile.env),
    };
    problem = null;
    editing = profile.id;
  }

  function apply() {
    const name = form.name.trim();
    if (!name) {
      problem = "Give the profile a name.";
      return;
    }
    const parsed = parseEnv(form.env);
    if ("error" in parsed) {
      problem = parsed.error;
      return;
    }
    const host = form.host.trim();
    if (form.kind === "ssh" && !host) {
      problem = "Give the SSH profile a host: a name from your SSH config or an address.";
      return;
    }
    const port = parsePort(form.port);
    if ("error" in port) {
      problem = port.error;
      return;
    }
    const folders = parseFolders(form.folders);
    if ("error" in folders) {
      problem = folders.error;
      return;
    }
    const size = parseFontSize(form.fontSize);
    if ("error" in size) {
      problem = size.error;
      return;
    }
    const base = {
      name,
      kind: form.kind,
      // What the kind does not use is left empty, so nothing stale is saved.
      host: form.kind === "ssh" ? host : null,
      user: form.kind === "ssh" ? form.user.trim() || null : null,
      port: form.kind === "ssh" ? port.port : null,
      distribution: form.kind === "wsl" ? form.distribution.trim() || null : null,
      shell: form.kind === "local" ? form.shell.trim() || null : null,
      workingDirectory: form.kind === "ssh" ? null : form.workingDirectory.trim() || null,
      folders: folders.folders,
      theme: form.theme || null,
      fontSize: size.size,
      fontFamily: form.fontFamily.trim() || null,
      cursorStyle: (form.cursorStyle || null) as CursorStyle | null,
      env: parsed.env,
    };
    if (editing === "") {
      const id = uniqueProfileId(name, profiles.map((profile) => profile.id));
      profiles = [...profiles, { id, ...base }];
    } else {
      profiles = profiles.map((profile) => (profile.id === editing ? { ...profile, ...base } : profile));
    }
    editing = null;
  }

  function remove(id: string) {
    profiles = profiles.filter((profile) => profile.id !== id);
    if (defaultProfile === id) defaultProfile = null;
  }
</script>

<div class="space-y-2">
  <div class="divide-y divide-line rounded-lg border border-line bg-panel-2" role="radiogroup" aria-label="Default profile">
    <div class="flex items-center gap-2.5 px-3 py-2 text-[12px]">
      <input
        type="radio"
        name="default-profile"
        checked={defaultProfile === null}
        onchange={() => (defaultProfile = null)}
        class="accent-accent"
        aria-label="Use the built-in profile by default"
      />
      <span class="min-w-0 flex-1">
        <span class="font-medium text-fg">Built-in</span>
        <span class="block truncate text-[10.5px] text-faint">The shell and folder set above</span>
      </span>
      {#if defaultProfile === null}
        <span class="rounded bg-accent-soft px-1.5 py-0.5 text-[10px] font-medium text-accent">default</span>
      {/if}
    </div>

    {#each profiles as profile (profile.id)}
      <div class="flex items-center gap-2.5 px-3 py-2 text-[12px]">
        <input
          type="radio"
          name="default-profile"
          checked={defaultProfile === profile.id}
          onchange={() => (defaultProfile = profile.id)}
          class="accent-accent"
          aria-label="Use {profile.name} by default"
        />
        <span class="min-w-0 flex-1">
          <span class="font-medium text-fg">{profile.name}</span>
          <span class="mono block truncate text-[10.5px] text-faint">{describeProfile(profile, themeNames)}</span>
        </span>
        {#if defaultProfile === profile.id}
          <span class="rounded bg-accent-soft px-1.5 py-0.5 text-[10px] font-medium text-accent">default</span>
        {/if}
        <button
          type="button"
          class="rounded px-1.5 py-0.5 text-[10.5px] text-muted transition-colors hover:bg-hover hover:text-fg"
          onclick={() => startEdit(profile)}
        >
          Edit
        </button>
        <button
          type="button"
          class="grid size-5 place-items-center rounded text-faint transition-colors hover:bg-hover hover:text-err"
          aria-label="Delete {profile.name}"
          onclick={() => remove(profile.id)}
        >
          <Icon name="trash" size={12} />
        </button>
      </div>
    {/each}
  </div>

  {#if editing !== null}
    <div class="space-y-2.5 rounded-lg border border-accent/50 bg-panel-2 p-3">
      <div>
        <label for="profile-name" class={small}>Name</label>
        <input
          id="profile-name"
          use:focusFirst
          type="text"
          maxlength="64"
          bind:value={form.name}
          class={field}
          placeholder="Work"
        />
      </div>
      <div>
        <label for="profile-kind" class={small}>Starts</label>
        <select id="profile-kind" bind:value={form.kind} class={field}>
          <option value="local">A shell on this computer</option>
          <option value="ssh">An SSH connection</option>
          {#if wslOffered}
            <option value="wsl">A WSL distribution</option>
          {/if}
        </select>
      </div>
      {#if form.kind === "ssh"}
        <div>
          <label for="profile-host" class={small}>Host</label>
          <input id="profile-host" type="text" list="termzip-ssh-hosts" spellcheck="false" autocomplete="off" bind:value={form.host} class={field} placeholder="build-box" />
          <datalist id="termzip-ssh-hosts">
            {#each sshHosts as host (host)}
              <option value={host}></option>
            {/each}
          </datalist>
        </div>
        <div class="grid grid-cols-2 gap-2.5">
          <div>
            <label for="profile-user" class={small}>User (empty: the one ssh uses)</label>
            <input id="profile-user" type="text" spellcheck="false" autocomplete="off" bind:value={form.user} class={field} />
          </div>
          <div>
            <label for="profile-port" class={small}>Port (empty: the one ssh uses)</label>
            <input id="profile-port" type="text" inputmode="numeric" placeholder="22" bind:value={form.port} class={field} />
          </div>
        </div>
        <p class="text-[10.5px] text-faint">
          TermZip runs <span class="mono">ssh</span> and stores no key, password or identity file: your agent and
          <span class="mono">~/.ssh/config</span> decide how you sign in. The other side is a plain terminal, so there are no
          command blocks (see the SSH guide to turn them on).
        </p>
      {:else if form.kind === "wsl"}
        <div>
          <label for="profile-distribution" class={small}>Distribution (empty: the default one)</label>
          <input id="profile-distribution" type="text" list="termzip-wsl-distributions" spellcheck="false" autocomplete="off" bind:value={form.distribution} class={field} placeholder="Ubuntu" />
          <datalist id="termzip-wsl-distributions">
            {#each distributions as name (name)}
              <option value={name}></option>
            {/each}
          </datalist>
        </div>
        <div>
          <label for="profile-directory" class={small}>Folder inside the distribution (empty: its home)</label>
          <input id="profile-directory" type="text" bind:value={form.workingDirectory} class={field} placeholder="~/code" />
        </div>
      {:else}
        <div>
          <label for="profile-shell" class={small}>Shell (empty: the built-in one)</label>
          <input id="profile-shell" type="text" list="termzip-shell-paths" bind:value={form.shell} class={field} />
        </div>
        <div>
          <label for="profile-directory" class={small}>Folder (empty: the built-in one)</label>
          <input id="profile-directory" type="text" bind:value={form.workingDirectory} class={field} />
        </div>
      {/if}
      <div>
        <label for="profile-folders" class={small}>Use this profile in these folders, one per line (optional)</label>
        <textarea
          id="profile-folders"
          rows="2"
          spellcheck="false"
          bind:value={form.folders}
          class="{field} resize-y"
          placeholder="~/work/api"
        ></textarea>
        <p class="mt-1 text-[10.5px] text-faint">
          A new tab that starts in one of them, or below, uses this profile when you did not pick another.
        </p>
      </div>
      <div class="grid grid-cols-2 gap-2.5">
        <div class="col-span-2">
          <label for="profile-theme" class={small}>Terminal theme (empty: the app's)</label>
          <select id="profile-theme" bind:value={form.theme} class={field}>
            <option value="">Same as the app</option>
            {#each groups as group (group.label)}
              <optgroup label={group.label}>
                {#each group.entries as item (item.theme.id)}
                  <option value={item.theme.id}>{item.theme.name}</option>
                {/each}
              </optgroup>
            {/each}
            {#if form.theme && !themes.entries.some((entry) => entry.theme.id === form.theme)}
              <option value={form.theme}>{form.theme} (missing)</option>
            {/if}
          </select>
        </div>
        <div>
          <label for="profile-font-size" class={small}>Font size (empty: the app's)</label>
          <input id="profile-font-size" type="text" inputmode="numeric" placeholder="Same" bind:value={form.fontSize} class={field} />
        </div>
        <div>
          <label for="profile-cursor" class={small}>Cursor</label>
          <select id="profile-cursor" bind:value={form.cursorStyle} class={field}>
            <option value="">Same as the app</option>
            <option value="block">Block</option>
            <option value="bar">Bar</option>
            <option value="underline">Underline</option>
          </select>
        </div>
        <div class="col-span-2">
          <label for="profile-font-family" class={small}>Font family (empty: the app's)</label>
          <input id="profile-font-family" type="text" bind:value={form.fontFamily} class={field} />
        </div>
      </div>
      <div>
        <label for="profile-env" class={small}>Environment, one KEY=value per line</label>
        <textarea
          id="profile-env"
          rows="3"
          spellcheck="false"
          bind:value={form.env}
          class="{field} resize-y"
          placeholder="AWS_PROFILE=work"
        ></textarea>
      </div>
      {#if problem}
        <p class="text-[11px] text-err" role="alert">{problem}</p>
      {/if}
      <div class="flex justify-end gap-2">
        <button
          type="button"
          class="rounded-md border border-line px-2.5 py-1 text-[11px] text-muted hover:bg-hover hover:text-fg"
          onclick={() => (editing = null)}
        >
          Cancel
        </button>
        <button
          type="button"
          class="rounded-md bg-accent px-2.5 py-1 text-[11px] font-semibold text-white hover:opacity-90"
          onclick={apply}
        >
          {editing === "" ? "Add profile" : "Update profile"}
        </button>
      </div>
    </div>
  {:else}
    <button
      type="button"
      class="flex items-center gap-1.5 rounded-md border border-dashed border-line px-2.5 py-1.5 text-[11px] text-muted transition-colors hover:border-accent hover:text-accent disabled:opacity-40"
      disabled={profiles.length >= MAX_PROFILES}
      onclick={startNew}
    >
      <Icon name="plus" size={12} />
      Add a profile
    </button>
  {/if}
  <p class="text-[10.5px] text-faint">
    A profile only needs what differs: an empty field uses the built-in profile or the app's setting. Its theme,
    font and cursor are for its terminals; the window around them keeps the app's theme. Environment variables
    reach new shells; the ones TermZip manages (TERM, ZDOTDIR, TERMZIP_…) cannot be set. A profile tied to
    folders is used by a new terminal that starts in one of them, or below. An SSH or WSL profile connects
    instead of starting a shell here.
  </p>
</div>
