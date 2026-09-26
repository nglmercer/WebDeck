//! WebDeck — Rust port of the Python application.
//!
//! Module layout mirrors `app/` 1:1: every `app/<path>/<module>.py` maps to
//! `src/app/<path>/<module>.rs` (each `__init__.py` maps to the parent
//! `mod.rs`). See `docs/MIGRATION_RUST.md` for the full mapping table,
//! per-module port status, and crate equivalences.
//!
//! `src/lib.rs` itself has no Python counterpart: it only exposes `app/` so
//! the three binaries (`webdeck`, `console`, `update`) and integration tests
//! can share one implementation.

// Scaffolding allowance: many ported APIs are not wired up yet while the
// migration is in progress. Remove once the port is complete.
#![allow(dead_code)]

pub mod app;
