//! Port of `app/utils/` — shared helpers.
//!
//! Each submodule maps 1:1 to its `app/utils/<module>.py` counterpart
//! (nested packages map to nested modules the same way).

pub mod args;
pub mod debug;
pub mod exit;
pub mod firewall;
pub mod get_local_ip;
pub mod global_variables;
pub mod is_opened;
pub mod kill_nircmd;
pub mod languages;
pub mod logger;
pub mod merge_dicts;
pub mod plugins;
pub mod qr;
pub mod restart;
pub mod settings;
pub mod show_error;
pub mod themes;
pub mod translate;
pub mod welcome_popup;
pub mod working_dir;
