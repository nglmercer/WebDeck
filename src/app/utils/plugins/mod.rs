//! Port of `app/utils/plugins/` (`load_plugins.py`).

pub mod load_plugins;

pub use load_plugins::{load_plugins, plugin_commands, register_plugin_commands, PluginFn};
