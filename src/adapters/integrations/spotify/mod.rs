//! Port of `app/buttons/spotify/`.

pub mod albums;
pub mod artists;
pub mod command_handler;
pub mod playlists;
pub mod songs;
pub mod utils;
pub mod volume;

pub use command_handler::handle_command;
