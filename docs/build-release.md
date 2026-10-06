# Build and release

Regenerate embedded JavaScript with `npm ci --prefix runtime` and `npm run build --prefix runtime` when runtime sources change. Node is a development tool; the product embeds JavaScript in its Rust binary. Build the UI with `npm ci --prefix frontend` and `npm run build --prefix frontend`. Debug builds (`cargo build --locked --bins` or `cargo run`) serve frontend files from disk with caching disabled; rebuilding the UI does not require recompiling Rust. Release builds (`cargo build --locked --release --bins`) automatically run the frontend build and embed its HTML, JavaScript, CSS, static icons and fallback translations into the binary. Run `npm ci --prefix frontend` first. Frontend source changes invalidate the release build, so rebuilding Rust embeds the current UI. Release binaries serve their embedded UI regardless of the working directory or any older frontend files on disk. Native Linux dependencies are listed in the [quickstart](../README.md).

After building the release executable, run `node tools/validation/embedded-frontend.mjs` to verify that its current UI loads outside the repository with frontend files missing or stale. This checks HTML, JavaScript, CSS, favicon, cache headers and Chromium rendering without executing button actions.

`cargo run --locked --bin package -- --dev` creates a labelled development portable ZIP containing WebDeck, console, updater, QR viewer, v2 defaults, translations, version metadata, application icons and the frontend bundle. Without `--dev`, the packager builds release binaries. Linux development staging strips debug symbols from copies, preserving the original binaries.

The updater selects verified newer maintainer v2 prereleases, checks SHA-256, confines extraction and retains a rollback journal. Quit WebDeck before installation. Run `cargo run --bin update -- --help` for check, fetch, install and rollback options. Automatic updates remain disabled; this checkout does not establish signed production distribution.

[README validation commands](../README.md#validation-and-packaging) cover contracts, Rust, frontend, browser acceptance and portable packaging.
