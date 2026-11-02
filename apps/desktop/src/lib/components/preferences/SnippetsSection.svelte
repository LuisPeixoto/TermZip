<script lang="ts">
  import type { Settings } from "../../api";
  import SnippetsEditor from "../SnippetsEditor.svelte";

  let { draft = $bindable() }: { draft: Settings } = $props();
</script>

<SnippetsEditor bind:snippets={draft.snippets} />
