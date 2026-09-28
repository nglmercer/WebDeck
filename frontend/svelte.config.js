import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

// Svelte islands mount into the existing page (see src/components/*.svelte).
// vitePreprocess gives `<script lang="ts">` the same Vite/TS handling as
// the rest of the project; component styles stay in the global
// `static/css/*` tree on purpose (no scoped `<style>` blocks), so the
// user-theme cascade keeps working unchanged.
export default {
  preprocess: vitePreprocess(),
};
