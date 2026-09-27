//! Port of `app/updater/updater.py`.
//!
//! - `requests` → `reqwest` (async fns), `tqdm` → log lines, `zipfile` → `zip`.
//! - `compare_versions` / `check_files` / `move_folder_content` are ported 1:1.
//! - Logging goes to the updater log file ([`updater_log`]), like Python's
//!   module-level `Logger(from_updater=True)`.
//! - Python uses `__main__`-time globals (`settings`, `wd_dir`, `update_dir`);
//!   Rust recomputes them from [`get_base_dir`] in each function.
//!
//! Split: [`versions`], [`files`], [`download`], [`apply`], [`admin`].
//!
//! [`updater_log`]: crate::app::utils::logger::updater_log
//! [`get_base_dir`]: crate::app::utils::working_dir::get_base_dir

mod admin;
mod apply;
mod download;
mod files;
mod versions;

pub use admin::{needs_admin_permissions, prepare_update_directory, request_admin_permissions};
pub use apply::check_updates;
pub use download::{download_and_extract, fetch_latest_release};
pub use files::{check_files, move_folder_content};
pub use versions::compare_versions;
