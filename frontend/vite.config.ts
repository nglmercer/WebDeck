import { svelte } from '@sveltejs/vite-plugin-svelte';
import { createLogger } from 'vite';
import { defineConfig } from 'vitest/config';

// `/static/*` URLs in index.html are served by the Rust server at runtime
// (ServeDir on repo `static/`), so Vite's "doesn't exist at build time,
// will remain unchanged to be resolved at runtime" is the intended outcome,
// not a bug. Filter exactly that message; everything else still warns.
const logger = createLogger();
for (const method of ['warn', 'warnOnce'] as const) {
  const original = logger[method].bind(logger);
  logger[method] = ((message: string, options?: unknown) => {
    if (
      message.includes("doesn't exist at build time") &&
      message.includes('/static/')
    ) {
      return;
    }
    (original as (message: string, options?: unknown) => void)(message, options);
  }) as typeof logger[typeof method];
}

// Svelte islands mount into the existing page alongside the hand-rolled
// micro-framework (src/framework/*); each migrated view deletes its
// imperative counterpart. Relative base so the Rust server can serve
// dist/ from any route prefix.
export default defineConfig({
  base: './',
  customLogger: logger,
  plugins: [svelte()],
  // Vitest resolves through node conditions by default, which picks
  // Svelte's server build (`mount` unavailable). Force the browser
  // build for tests only; dev/build keep Vite's client defaults.
  ...(process.env.VITEST ? { resolve: { conditions: ['browser'] } } : {}),
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    target: 'es2020',
  },
  server: {
    port: 5173,
    // API + static assets stay on the Rust server during `npm run dev`.
    proxy: {
      '^/(api|send-data|usage|save_config|COMPLETE_save_config|save_single_button|save_buttons_only|get_config|upload_folderpath|upload_filepath|upload_file|create_folder|\\.config|static|socket\\.io)':
        'http://127.0.0.1:59997',
    },
  },
  test: {
    environment: 'happy-dom',
    environmentOptions: { happyDOM: { settings: { disableCSSFileLoading: true, handleDisabledFileLoadingAsSuccess: true } } },
    include: ['src/**/*.test.ts'],
  },
});
