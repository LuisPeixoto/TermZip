import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

export default defineConfig({
  // The stores are `.svelte.ts` modules (runes), so they need the Svelte compiler to be tested.
  plugins: [svelte({ hot: false })],
  resolve: { conditions: ["browser"] },
  test: {
    environment: "node",
    include: ["tests/**/*.spec.ts"],
  },
});
