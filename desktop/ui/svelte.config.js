/**
 * Svelte configuration: vitePreprocess for TypeScript in components. No
 * SvelteKit; this is a plain Vite + Svelte SPA inside Tauri.
 */

import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

export default {
  preprocess: vitePreprocess(),
};
