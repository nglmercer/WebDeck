//! Port of `app/buttons/system/`.

pub mod command_handler;
pub mod opendir;
pub mod openfile;

pub use command_handler::handle_command;
