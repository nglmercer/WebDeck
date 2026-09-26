//! Port of the `app/` package.
//!
//! 1:1 mapping: `server.py` → [`server`], `tray.py` → [`tray`],
//! `buttons/` → [`buttons`], `on_start/` → [`on_start`],
//! `updater/` → [`updater`], `utils/` → [`utils`].

pub mod buttons;
pub mod on_start;
pub mod server;
pub mod tray;
pub mod updater;
pub mod utils;
