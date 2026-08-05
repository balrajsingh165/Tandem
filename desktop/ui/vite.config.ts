/**
 * Vite configuration: Svelte plugin, dev-server port for tauri dev, and build
 * output consumed by the Tauri bundler.
 */

import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      // Rust build output holds hundreds of thousands of files; watching it
      // exhausts the file-watcher handles and kills the dev server.
      ignored: ['**/src-tauri/target/**', '**/target/**'],
    },
  },
  build: {
    target: 'es2022',
    outDir: 'dist',
    emptyOutDir: true,
    sourcemap: true,
  },
  resolve: {
    alias: {
      $lib: new URL('./src/lib', import.meta.url).pathname,
    },
  },
});
