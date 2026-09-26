//! Port of `app/updater/`.

pub mod check;
pub mod updater;

pub use check::check_for_updates;
pub use updater::check_files;
