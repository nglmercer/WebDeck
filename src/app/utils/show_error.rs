//! Port of `app/utils/show_error.py`.
//!
//! Logging behavior is ported 1:1. The tkinter dialogs
//! (`messagebox.showerror` / GitHub-issue / copy-to-clipboard prompts) and the
//! `MessageBoxW` fallback are TODO — planned via `rfd` native dialogs (see
//! `docs/MIGRATION_RUST.md`). Until then the full message goes to the log and
//! stderr so no error is ever swallowed.

use std::fmt::Debug;

use crate::app::utils::logger::log;

/// Port of `show_error`.
pub fn show_error(
    message: Option<&str>,
    title: &str,
    error: bool,
    exception: Option<&dyn Debug>,
) {
    let full_message: String = match (message, exception) {
        (Some(message), Some(exception)) => {
            log().exception(exception, Some(message), false, true, true);
            format!("{message}\n\n\n{exception:?}\n\n{exception:#?}")
        }
        (None, Some(exception)) => {
            log().exception(exception, None, false, true, true);
            format!("{exception:?}\n\n{exception:#?}")
        }
        (Some(message), None) => {
            log().error(message);
            message.to_string()
        }
        (None, None) => {
            log().error("Unknown error");
            "Unknown error".to_string()
        }
    };

    if error {
        // TODO(port): native error dialog (rfd) with "open GitHub issue" and
        // "copy to clipboard" prompts, mirroring the tkinter flow.
        eprintln!("=== {title} ===\n{full_message}");
        log().warning("Native error dialog not ported yet; error printed to stderr instead");
    } else {
        // TODO(port): MessageBoxW equivalent (rfd message dialog).
        eprintln!("=== {title} ===\n{full_message}");
    }
}
