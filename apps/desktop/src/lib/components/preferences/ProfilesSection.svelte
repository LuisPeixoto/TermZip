<script lang="ts">
  import type { Settings } from "../../api";
  import ProfilesEditor from "../ProfilesEditor.svelte";

  let { draft = $bindable() }: { draft: Settings } = $props();
</script>

<ProfilesEditor bind:profiles={draft.profiles} bind:defaultProfile={draft.defaultProfile} />
