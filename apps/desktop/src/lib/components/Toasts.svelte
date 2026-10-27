<script lang="ts">
  import { app } from "../stores/app.svelte";

  const styles: Record<string, string> = {
    info: "border-line text-fg",
    ok: "border-ok/40 text-ok",
    warn: "border-warn/40 text-warn",
    err: "border-err/40 text-err",
  };
</script>

<div
  class="pointer-events-none fixed bottom-12 right-4 z-50 flex flex-col gap-2"
  role="status"
  aria-live="polite"
>
  {#each app.toasts as toast (toast.id)}
    <div
      class="pointer-events-auto rounded-lg border bg-panel px-3 py-2 text-[12px] shadow-pop {styles[toast.kind] ?? styles.info}"
    >
      {toast.message}
    </div>
  {/each}
</div>