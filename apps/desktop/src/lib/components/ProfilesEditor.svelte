<script lang="ts">
  import Icon from "./Icon.svelte";
  import type { CursorStyle, Profile } from "../api";
  import { MAX_PROFILES, describeProfile, formatEnv, parseEnv, parseFontSize, uniqueProfileId } from "../profiles";
  import { themes } from "../stores/themes.svelte";
  import { groupThemes } from "../theme";

  let {
    profiles = $bindable(),
    defaultProfile = $bindable(),
  }: { profiles: Profile[]; defaultProfile: string | null } = $props();

  // `null` is the list, `""` a new profile, anything else the id being edited.
  let editing = $state<string | null>(null);
  const blankForm = { name: "", shell: "", workingDirectory: "", theme: "", fontSize: "", fontFamily: "", cursorStyle: "", env: "" };
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
      shell: profile.shell ?? "",
      workingDirectory: profile.workingDirectory ?? "",
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
    const size = parseFontSize(form.fontSize);
    if ("error" in size) {
      problem = size.error;
      return;
    }
    const base = {
      name,
      shell: form.shell.trim() || null,
      workingDirectory: form.workingDirectory.trim() || null,
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
  <ul class="divide-y divide-line rounded-lg border border-line bg-panel-2" role="radiogroup" aria-label="Default profile">
    <li class="flex items-center gap-2.5 px-3 py-2 text-[12px]">
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
    </li>

    {#each profiles as profile (profile.id)}
      <li class="flex items-center gap-2.5 px-3 py-2 text-[12px]">
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
      </li>
    {/each}
  </ul>

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
        <label for="profile-shell" class={small}>Shell (empty: the built-in one)</label>
        <input id="profile-shell" type="text" list="termzip-shell-paths" bind:value={form.shell} class={field} />
      </div>
      <div>
        <label for="profile-directory" class={small}>Folder (empty: the built-in one)</label>
        <input id="profile-directory" type="text" bind:value={form.workingDirectory} class={field} />
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
    reach new shells; the ones TermZip manages (TERM, ZDOTDIR, TERMZIP_…) cannot be set.
  </p>
</div>
