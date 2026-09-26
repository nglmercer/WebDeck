//! Port of `app/utils/welcome_popup.py`.
//!
//! The `settings.show_popup` gate is ported 1:1. The customtkinter window is
//! TODO — planned via a small `wry`/native dialog (see
//! `docs/MIGRATION_RUST.md`). Until then this logs and returns so startup is
//! never blocked.

use crate::app::utils::{languages::text, logger::log, settings::get_config};

/// Port of `show_popup`.
pub fn show_popup() {
    let config = get_config::get_config(false, false);
    let show = config
        .get("settings")
        .and_then(|s| s.get("show_popup"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if !show {
        return;
    }

    // TODO(port): welcome popup window (customtkinter equivalent).
    log().info(&format!(
        "{}: {}",
        text(Some("welcome_message_window_title"), None),
        text(Some("welcome_message_label_1"), None),
    ));
}
