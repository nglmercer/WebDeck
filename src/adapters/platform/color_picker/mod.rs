//! Port of `app/buttons/color_picker/`.

pub mod command_handler;
pub mod get_arg;
pub mod get_color_name;
pub mod get_mouse_pixel_color;
pub mod notification;

pub use command_handler::handle_command;
