//! Port of `app/buttons/window/`.

pub mod close_window;
pub mod find_window;
pub mod focus_window;
pub mod get_focused;

pub use find_window::get_window_by_name as get_by_name;
pub use close_window::close_window as close;
pub use get_focused::get_focused_window as get_focused;
pub use focus_window::bring_window_to_front;
