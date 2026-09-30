//! Compatibility imports; native and integration implementations have one owner.
pub mod commands;
pub use crate::adapters::integrations::{exec, fetch, obs, spotify};
pub use crate::adapters::platform::{audio, color_picker, soundboard, system, usage, window};
pub use commands::handle_command;
