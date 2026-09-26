//! Port of `app/buttons/`.
//!
//! `app/buttons/__init__.py` is empty in Python (callers rely on the
//! `from package import submodule` fallback); Rust re-exports the command
//! entry point explicitly instead.

pub mod audio;
pub mod color_picker;
pub mod commands;
pub mod exec;
pub mod obs;
pub mod soundboard;
pub mod spotify;
pub mod system;
pub mod usage;
pub mod window;

pub use commands::handle_command;
