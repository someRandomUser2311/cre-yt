import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Unit tests run in Node against pure logic and Svelte runes stores.
// The Svelte plugin lets us import `.svelte.ts` files (runes compiled).
export default defineConfig({
  plugins: [svelte({ hot: false })],
  test: {
    include: ["src/**/*.{test,spec}.{js,ts}"],
    environment: "node",
  },
});
