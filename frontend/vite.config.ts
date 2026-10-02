import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';
export default defineConfig({
  plugins: [svelte()],
  base: '/',
  ...(process.env.VITEST ? { resolve: { conditions: ['browser'] } } : {}),
  build: { target: 'es2022' },
  server: {
    proxy: {
      '/api': 'http://127.0.0.1:5000',
      '/static': 'http://127.0.0.1:5000',
      '/socket.io': { target: 'http://127.0.0.1:5000', ws: true },
    },
  },
  test: { environment: 'happy-dom', include: ['src/**/*.test.ts'] },
});
