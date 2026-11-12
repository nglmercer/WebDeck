import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';
const generator = fileURLToPath(new URL('../tools/i18n/generate.mjs', import.meta.url));
const locales = fileURLToPath(new URL('./locales/', import.meta.url));
function generateTranslations() {
  execFileSync(process.execPath, [generator], { stdio: 'pipe' });
}
export default defineConfig({
  plugins: [
    {
      name: 'webdeck-translations',
      buildStart: generateTranslations,
      configureServer(server) {
        server.watcher.add(locales);
        const update = (_event: string, file: string) => {
          if (!path.relative(locales, file).startsWith('..') && file.endsWith('.ts')) {
            try {
              generateTranslations();
              server.ws.send({ type: 'full-reload' });
            } catch (error) {
              server.config.logger.error(String(error));
              server.ws.send({
                type: 'error',
                err: { message: String(error), stack: '', plugin: 'webdeck-translations' },
              });
            }
          }
        };
        server.watcher.on('all', update);
        server.httpServer?.once('close', () => server.watcher.off('all', update));
      },
    },
    svelte(),
    {
      name: 'webdeck-contract-version',
      transformIndexHtml() {
        const version = createHash('sha256')
          .update(readFileSync(new URL('../contracts/v2.schema.json', import.meta.url)))
          .digest('hex');
        return [
          { tag: 'meta', attrs: { name: 'webdeck-contract', content: version }, injectTo: 'head' },
        ];
      },
    },
  ],
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
