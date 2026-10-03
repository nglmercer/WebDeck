import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

// The v2 app imports its global stylesheet from src/main.ts.
export default {
  preprocess: vitePreprocess(),
};
