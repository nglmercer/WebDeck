# V2-only implementation report

**Outcome: incomplete, with unfinished code blockers.** This is not a completed
migration awaiting only external platform validation. The single PR is a draft.

Starting commit: `d86e5c730393854da06c16ad4791e5249b6842d8` (`v2`, clean tree).
Implementation branch: `codex/v2-only-migration`, targeting `v2`. All changes
from this work are on that branch; no deprecated-branch changes or releases.
The PR's head commit identifies the final code/evidence snapshot.

Implemented: version-independent paired authorization; private controller boot;
retirement of `/api/boot`, `/send-data` and the root command namespace; shared v2
HTTP/socket command transport; correlated no-replay socket observation with
uncertain outcomes on timeout/disconnect; owned accepted-work lifecycles while
queued/running; schema-version rejection without conversion or data deletion;
mandatory client mutation revisions and read-only config snapshots; transactional
folder creation and application-owned layout/settings publication; prerelease
identity and constrained v2 updater selection with automatic updates disabled.

[STATUS.md](STATUS.md) contains the surface-by-surface migration map, consumers,
evidence, incomplete models/adapters/plugins/scripts/tooling and separate release
gates. This change does **not** remove all compatibility or provide complete typed
action/config contracts. It must not be merged as a completed v2-only product.

## Checks run

Commands ran on Linux with the installed native library/toolchain selectors
from `/workspace/.webdeck-tools/env.sh`. Independent exit statuses, timings and
logs are in [evidence/v2-only/](evidence/v2-only/). The baseline existing Rust
suite and 379 frontend tests passed. The first plain-shell Cargo attempt lacked
PATH setup; it was rerun using the installed toolchain. Intermediate compile,
HTTP-fixture and type failures were repaired rather than suppressed.

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Pass |
| `node tools/contracts/generate.mjs --check` | Pass; existing source/artifacts agree, not proof of complete contracts |
| `cargo test --locked --all-targets` | Pass: 140 tests, 2 opt-in tests ignored |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | Pass |
| `npm ci --prefix frontend` | Pass |
| `npm ci --prefix tools/component-check` | Pass |
| `npm --prefix frontend run check:contracts` | Pass |
| `npm --prefix frontend run check:components` | Pass: 0 errors/warnings |
| `npm --prefix frontend run typecheck` | Pass |
| `npm --prefix frontend run knip` | Pass |
| `npm --prefix frontend test` | Pass: 383 tests in 54 files |
| `npm --prefix frontend run build` | Pass |
| `cargo build --locked --bin webdeck` | Pass |
| `npm --prefix frontend exec -- playwright install chromium` | Pass; acceptance used installed `/usr/bin/chromium` |
| `(cd frontend && npm run test:acceptance)` | Pass: 4 real HTTP/Socket.IO browser tests with fake native effects |
| `cargo run --locked --bin package -- --dev` | Pass: actual versioned Linux development archive |
| `node tools/validation/portable.mjs dist/WebDeck-2.0.0-alpha.1-linux-x86_64-dev-portable.zip` | Pass: extracted-artifact checksum, executable permissions, boot/assets/catalog and Chromium deck smoke |
| `WEBDECK_PORTABLE_ARTIFACT=/workspace/WebDeck/dist/WebDeck-2.0.0-alpha.1-linux-x86_64-dev-portable.zip cargo test --locked --test v2_update portable_archive -- --ignored` | Pass: actual archive upgrade and real updater rollback preserve user data |
| `node tools/validation/v2-only.mjs --json` | **Fail**: remaining production compatibility surfaces; completion gate enabled in CI |
| `git diff --check` | Pass |

The initial development package contained debug symbols and exceeded the
updater archive size limit. Linux development packaging now strips debug
symbols only from staged copies, retaining original build binaries. The corrected
archive was used for smoke and upgrade/rollback. No guessed or historical archive
was substituted. Its digest/profile are recorded in
[evidence/v2-only/portable.json](evidence/v2-only/portable.json).

The two default ignored Rust checks are network translation and the opt-in
packaged-artifact test. The latter was separately run against the generated
archive. Live translation was not run. Windows CI, live desktop/hardware actions,
OBS/Spotify credentials, real representative plugins/scripts and signed release
packaging were not available/run in this Linux workspace. No hardware parity or
stable-release readiness is inferred from fakes or development artifact smoke.

## Remaining code blockers and guard

The guard is a scoped production-source gate: historical documents and rejection
tests may retain old names. It detects legacy parsers/facades/delimiters, remaining
old application routes, loose string-command requests, generation/security switches,
v1 conversion and legacy asset references. Its detailed nonzero output is retained
in `evidence/v2-only/migration-guard.json`. The ordinary checks passing while this
completion gate fails is deliberate evidence of incomplete migration, not a green
completion claim.

Remaining work includes generated discriminated actions and complete validated
config/deck/button/catalog/upload models; stable folder/button IDs and ordering;
remaining HTTP and asset consumers; typed native/integration adapter arguments
and truthful explicit results; safe bounded nested invocation; v2 plugin and
script-source contracts/examples; full frontend feature-state/runtime validation;
replacement of positive compatibility fixtures and complete acceptance coverage.
Performance comparison against the frozen pre-migration application was not run:
the updated harness requires v2 boot/command interfaces. No performance improvement
or budget is claimed.

Internal config writers/global state still need consolidation with their original
read snapshots. The map identifies each remaining route and caller family.

All these code blockers must be resolved before claiming v2-only completion.
Separate Windows, native, credential, security-review and verified distribution
release gates remain after that work. Automatic update polling stays disabled.
