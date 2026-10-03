# Build and release

Regenerate embedded JavaScript with `npm ci --prefix runtime` and `npm run build --prefix runtime` when runtime sources change. Node is a development tool; the product embeds JavaScript in its Rust binary. Build the UI with `npm ci --prefix frontend` and `npm run build --prefix frontend`. Build Rust binaries with `cargo build --locked --bins`. Native Linux dependencies are listed in the [quickstart](../README.md).

`cargo run --locked --bin package -- --dev` creates a labelled development portable ZIP containing WebDeck, console, updater, QR viewer, v2 defaults, translations, version metadata, application icons and the frontend bundle. Without `--dev`, the packager builds release binaries. Linux development staging strips debug symbols from copies, preserving the original binaries.

The updater selects verified newer maintainer v2 prereleases, checks SHA-256, confines extraction and retains a rollback journal. Quit WebDeck before installation. Run `cargo run --bin update -- --help` for check, fetch, install and rollback options. Automatic updates remain disabled; this checkout does not establish signed production distribution.

[README validation commands](../README.md#validation-and-packaging) cover contracts, Rust, frontend, browser acceptance and portable packaging.
