import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

export default defineConfig({
  // The stores are `.svelte.ts` modules (runes), so they need the Svelte compiler to be tested.
  plugins: [svelte({ hot: false })],
  resolve: { conditions: ["browser"] },
  test: {
    // A DOM environment compiles what the tests import for the client. In the plain node
    // environment `$effect` compiles to nothing, so a test of reactive code would pass without
    // ever running its effects.
    environment: "happy-dom",
    include: ["tests/**/*.spec.ts"],
  },
});
