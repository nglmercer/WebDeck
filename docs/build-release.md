# Build & Release

## Dev loop

```sh
# Backend (from repo root)
cargo build
cargo test            # 92 lib + integration tests
cargo fmt --check
cargo clippy --all-targets

# Frontend (from frontend/)
npm ci
npm test              # vitest, 298 tests
npm run typecheck     # TypeScript
npm ci --prefix ../tools/component-check
npm run check:components # Svelte component diagnostics
npm run build         # → frontend/dist/ (required by GET /)
```

Router regression tests drive the real `app_router` via `oneshot`; disk-config
tests use `$WEBDECK_CONFIG_DIR` + a shared temp dir so the real
`.config/config.json` is never touched.

## Binaries

`cargo build --release --bin webdeck --bin update --bin webdeck-qr`
produces the three shipped binaries (`webdeck` → `WebDeck`,
`update`, `webdeck-qr`). `console` is dev-only and never shipped.
`package` is the packager itself.

Linux GUI builds need webkit2gtk + gtk system libraries; the tray uses ksni
(pure Rust, native on KDE/Wayland, no libappindicator). Linux screenshot
capture uses grim + ashpd + xcb — never the `screenshots` crate (its
dbus/vendored libdbus lacks `HAVE_POLL` → SIGABRT on fd ≥ 1024).

## Packaging (`src/bin/package.rs`)

`cargo run --release --bin package` — replacement for `setup.py` + `build.bat`.
Produces `dist/WebDeck-<os>-<arch>-portable.zip` with a top-level `WebDeck/`
folder (the layout the updater extracts):

1. `read_version()` from `webdeck/version.json` (`versions[0].version`).
2. Release build (three binaries above).
3. `ensure_frontend_dist()` — `npm ci` + `npm run build` when
   `frontend/dist/index.html` is missing (Python's ignore rules silently
   shipped frozen apps with no web UI; Rust always includes it).
4. `download_nircmd()` — fetch `nircmd.zip`, extract `nircmd.exe` to
   `temp/` (skipped when present), staged into `lib/`.
5. `stage_tree()` → `temp/portable/WebDeck/`: binaries + `webdeck/`,
   `static/`, `frontend/dist/`, `README*` → `docs/`, `lib/nircmd.exe`.
6. `sign_binaries()` — same `signtool` invocation on Windows; failures warn
   and continue.
7. `zip_stage()` — deflated zip, `0o755` on Unix binaries.

`bdist_msi` has no cargo equivalent and stays a manual WiX step; the portable
zip is the shippable + auto-update artifact.

## Updater (`src/app/updater/` + `src/bin/update.rs`)

Two phases:

- `check::check_for_updates()` — runs inside `on_start()` when
  `(auto_updates || --force-update) && !--no-auto-update`: compares
  `webdeck/version.json` against the release feed for `update_channel` in
  `update_repo`, then hands off to the `update` binary.
- `updater/` (`admin`, `apply`, `download`, `files`, `versions`) — download
  the portable zip, apply with elevation when needed (`admin`), reconcile
  files (`files`, incl. `check_files()` at every start), parse versions
  (`versions`).

`version.json` entries carry `updated_files` / `deleted_files` /
`renamed_files` directives plus per-locale changelogs (`en`/`fr`:
`updates`, `minor_enhancements`, `fixes`, `nobody_cares`).
Current version: 1.8.7.


## V2 distribution and recovery

See [v2 status](v2/STATUS.md) for the current architecture and release gates.
`cargo run --locked --bin package` rebuilds the frontend and native release
binaries and writes a platform ZIP plus `.zip.sha256`. `--dev` creates an
explicitly named unsigned development ZIP. Windows packaging requires
`WEBDECK_NIRCMD_SHA256` obtained independently from a verified vendor archive;
release signing must succeed for all three shipped binaries.

The updater requires GitHub HTTPS assets with a trusted release SHA-256,
validates the complete archive, stages it privately, and journals originals
before publication. Failure restores originals. A committed backup is kept.
Close WebDeck before manually recovering an installation:

```sh
/path/to/update --rollback /path/to/backup --destination /path/to/installation
```

Use the backup created by that installation only; keep it until recovery has
been checked. User configuration is excluded from update packages.
The local backup directory and installation are trusted administrator-owned
paths. Windows locked executables cause a safe failure; validate installation
and rollback on Windows before release. The automatic launcher currently
stops the application before the updater checks the download; a failed
preflight requires manually restarting the unchanged application.
