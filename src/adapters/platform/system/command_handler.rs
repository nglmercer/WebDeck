//! Port of `app/buttons/system/command_handler.py`.

use crate::adapters::platform::system::{opendir::opendir, openfile::openfile};

/// Port of `handle_command`.
pub fn handle_command(message: &str) {
    if message.starts_with("/openfolder") || message.starts_with("/opendir") {
        opendir(message);
    } else if message.starts_with("/openfile") || message.starts_with("/start") {
        let path = message
            .replacen("/openfile", "", 1)
            .replacen("/start", "", 1);
        openfile(path.trim());
    }
}
