import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';
export default defineConfig({
  plugins: [svelte(), {
    name: 'webdeck-contract-version',
    transformIndexHtml() {
      const version = createHash('sha256').update(readFileSync(new URL('../contracts/v2.schema.json', import.meta.url))).digest('hex');
      return [{ tag: 'meta', attrs: { name: 'webdeck-contract', content: version }, injectTo: 'head' }];
    },
  }],
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
