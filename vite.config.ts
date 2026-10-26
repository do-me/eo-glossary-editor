import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
  plugins: [tailwindcss(), svelte()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  envPrefix: ['VITE_'],
  build: {
    target: 'es2022',
    rollupOptions: {
      output: { manualChunks: { editor: ['@milkdown/crepe/builder', '@milkdown/kit'] } },
    },
  },
});
