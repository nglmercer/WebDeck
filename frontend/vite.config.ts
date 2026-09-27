import { defineConfig } from 'vite';

// Zero runtime dependencies: the app is a hand-rolled micro-framework
// (src/framework/*). Vite is build tooling only. Relative base so the
// Rust server can serve dist/ from any route prefix.
export default defineConfig({
  base: './',
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
});
