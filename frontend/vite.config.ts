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

// Zero runtime dependencies: the app is a hand-rolled micro-framework
// (src/framework/*). Vite is build tooling only. Relative base so the
// Rust server can serve dist/ from any route prefix.
export default defineConfig({
  base: './',
  customLogger: logger,
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    target: 'es2020',
  },
  server: {
    port: 5173,
    // API + static assets stay on the Rust server during `npm run dev`.
    proxy: {
      '^/(send-data|usage|save_config|COMPLETE_save_config|save_single_button|save_buttons_only|get_config|upload_folderpath|upload_filepath|upload_file|create_folder|\\.config|static|socket\\.io)':
        'http://127.0.0.1:59997',
    },
  },
  test: {
    environment: 'happy-dom',
    include: ['src/query/**/*.test.ts', 'src/components/**/*.test.ts', 'src/views/**/*.test.ts'],
  },
});
