<script lang="ts">
  import Icon from "./Icon.svelte";
  import { api, type Snippet, type SnippetParameter } from "../api";
  import { MAX_SNIPPETS, describeScope, uniqueSnippetId } from "../snippets";

  let { snippets = $bindable() }: { snippets: Snippet[] } = $props();

  // `null` is the list, `""` a new snippet, anything else the id being edited.
  let editing = $state<string | null>(null);
  const blankForm = { name: "", command: "", description: "", folder: "" };
  let form = $state({ ...blankForm });
  let problem = $state<string | null>(null);
  // What the backend says about the command as it is typed: the values it asks for, or why it cannot be saved.
  let checked = $state<{ parameters: SnippetParameter[] } | { error: string } | null>(null);

  const field =
    "mono w-full rounded-lg border border-line bg-panel-2 px-2.5 py-1.5 text-[11.5px] outline-none focus:border-accent";
  const small = "mb-1 block text-[10.5px] font-medium text-muted";

  // The form opens at the bottom of a scrolling dialog: bring it into view and start typing.
  function focusFirst(node: HTMLInputElement) {
    node.focus();
    node.scrollIntoView({ block: "nearest" });
  }

  // The same rules as saving, asked of the backend a moment after the typing stops.
  $effect(() => {
    const command = form.command;
    if (editing === null || command.trim() === "") {
      checked = null;
      return;
    }
    const timer = setTimeout(() => {
      api
        .checkSnippet(command)
        .then((parameters) => {
          if (form.command === command) checked = { parameters };
        })
        .catch((error) => {
          if (form.command === command) checked = { error: String(error) };
        });
    }, 200);
    return () => clearTimeout(timer);
  });

  function startNew() {
    form = { ...blankForm };
    problem = null;
    checked = null;
    editing = "";
  }

  function startEdit(snippet: Snippet) {
    form = {
      name: snippet.name,
      command: snippet.command,
      description: snippet.description ?? "",
      folder: snippet.folder ?? "",
    };
    problem = null;
    editing = snippet.id;
  }

  function apply() {
    const name = form.name.trim();
    if (!name) {
      problem = "Give the snippet a name.";
      return;
    }
    if (!form.command.trim()) {
      problem = "Write the command.";
      return;
    }
    if (checked && "error" in checked) {
      problem = checked.error;
      return;
    }
    const base = {
      name,
      command: form.command.trim(),
      description: form.description.trim() || null,
      folder: form.folder.trim() || null,
    };
    if (editing === "") {
      const id = uniqueSnippetId(name, snippets.map((snippet) => snippet.id));
      snippets = [...snippets, { id, ...base }];
    } else {
      snippets = snippets.map((snippet) => (snippet.id === editing ? { ...snippet, ...base } : snippet));
    }
    editing = null;
  }

  function remove(id: string) {
    snippets = snippets.filter((snippet) => snippet.id !== id);
  }
</script>

<div class="space-y-2">
  {#if snippets.length > 0}
    <ul class="divide-y divide-line rounded-lg border border-line bg-panel-2" aria-label="Snippets">
      {#each snippets as snippet (snippet.id)}
        <li class="flex items-center gap-2.5 px-3 py-2 text-[12px]">
          <span class="min-w-0 flex-1">
            <span class="font-medium text-fg">{snippet.name}</span>
            <span class="mono block truncate text-[10.5px] text-faint">{snippet.command}</span>
            <span class="block truncate text-[10.5px] text-faint">{describeScope(snippet)}</span>
          </span>
          <button
            type="button"
            class="rounded px-1.5 py-0.5 text-[10.5px] text-muted transition-colors hover:bg-hover hover:text-fg"
            onclick={() => startEdit(snippet)}
          >
            Edit
          </button>
          <button
            type="button"
            class="grid size-5 place-items-center rounded text-faint transition-colors hover:bg-hover hover:text-err"
            aria-label="Delete {snippet.name}"
            onclick={() => remove(snippet.id)}
          >
            <Icon name="trash" size={12} />
          </button>
        </li>
      {/each}
    </ul>
  {:else if editing === null}
    <p class="rounded-lg border border-dashed border-line px-3 py-4 text-center text-[11.5px] text-faint">
      No snippets yet. A snippet is a command you want to type again, with values you fill in each time.
    </p>
  {/if}

  {#if editing !== null}
    <div class="space-y-2.5 rounded-lg border border-accent/50 bg-panel-2 p-3">
      <div>
        <label for="snippet-name" class={small}>Name</label>
        <input id="snippet-name" use:focusFirst type="text" maxlength="64" bind:value={form.name} class={field} placeholder="Check out a branch" />
      </div>
      <div>
        <label for="snippet-command" class={small}>Command, on one line</label>
        <input
          id="snippet-command"
          type="text"
          spellcheck="false"
          autocomplete="off"
          bind:value={form.command}
          class={field}
          placeholder="git checkout {'{{branch:main}}'}"
        />
        {#if checked && "error" in checked}
          <p class="mt-1 text-[10.5px] text-err" role="alert">{checked.error}</p>
        {:else if checked && checked.parameters.length > 0}
          <p class="mt-1 text-[10.5px] text-faint">
            Asks for
            {#each checked.parameters as parameter, at (parameter.name)}
              {at > 0 ? ", " : ""}<span class="mono text-fg">{parameter.name}</span>{parameter.default !== null
                ? ` (starts as “${parameter.default}”)`
                : ""}
            {/each}
          </p>
        {/if}
      </div>
      <div>
        <label for="snippet-description" class={small}>Description (optional)</label>
        <input id="snippet-description" type="text" maxlength="200" bind:value={form.description} class={field} />
      </div>
      <div>
        <label for="snippet-folder" class={small}>Only in this folder and below (empty: everywhere)</label>
        <input id="snippet-folder" type="text" bind:value={form.folder} class={field} placeholder="~/work/app" />
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
          {editing === "" ? "Add snippet" : "Update snippet"}
        </button>
      </div>
    </div>
  {:else}
    <button
      type="button"
      class="flex items-center gap-1.5 rounded-md border border-dashed border-line px-2.5 py-1.5 text-[11px] text-muted transition-colors hover:border-accent hover:text-accent disabled:opacity-40"
      disabled={snippets.length >= MAX_SNIPPETS}
      onclick={startNew}
    >
      <Icon name="plus" size={12} />
      Add a snippet
    </button>
  {/if}
  <p class="text-[10.5px] text-faint">
    Press {"{{name}}"} where a value goes, or {"{{name:default}}"} to start it filled in; write \{"{{"} for a literal
    pair of braces. A snippet is typed at the prompt and never run: you read it, edit it and press Enter. Choose one
    from the command palette (the Insert a snippet shortcut lists only snippets).
  </p>
</div>
