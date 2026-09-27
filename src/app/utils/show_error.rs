//! Port of `app/utils/show_error.py`.
//!
//! Logging behavior is ported 1:1. The tkinter dialogs map to `rfd` native
//! dialogs with the same flow: error box → "open GitHub issue?" prompt →
//! (elif) "copy to clipboard?" prompt. `error=false` maps to a plain OK box
//! (the `MessageBoxW` equivalent). The full message is also printed to
//! stderr so headless runs (where dialogs cannot show) never swallow errors.

use std::fmt::Debug;

use crate::app::utils::{languages::text, logger::log};

/// Build the GitHub issue URL exactly like Python (title + templated body).
pub fn github_issue_url(title: &str, exception: &str, github_message: &str) -> String {
    let issue_title = format!("[AUTO] {title} - {exception}").replace(' ', "%20");
    let issue_body = format!(
        "**Describe the bug**     <!-- (edit this) -->\n\
         A clear and concise description of what the bug is.\n\n\
         **Steps to reproduce**     <!-- (example) -->\n\
         1. Go to '...'\n\
         2. Click on '....'\n\
         3. Scroll down to '....\n\n\n\
         ## The error\n\
         {github_message}"
    )
    .replace('\n', "%0A")
    .replace(' ', "%20")
    .replace('#', "%23");
    format!(
        "https://github.com/Lenochxd/WebDeck/issues/new?labels=bug&template=bug_report.md&title={issue_title}&body={issue_body}"
    )
}

fn ask_yes_no(title: &str, message: &str) -> bool {
    rfd::MessageDialog::new()
        .set_title(title)
        .set_description(message)
        .set_buttons(rfd::MessageButtons::YesNo)
        .show()
        == rfd::MessageDialogResult::Yes
}

/// Port of `show_error`.
///
/// NOTE: blocks on native dialogs — async callers must invoke via
/// `tokio::task::block_in_place`.
pub fn show_error(message: Option<&str>, title: &str, error: bool, exception: Option<&dyn Debug>) {
    let (full_message, github_message) = match (message, exception) {
        (Some(message), Some(exception)) => {
            log().exception(exception, Some(message), false, true, true);
            (
                format!("{message}\n\n\n{exception:?}\n\n{exception:#?}"),
                format!("`{message}`\n\n\n```\n{exception:?}\n```\n\n```{exception:#?}```"),
            )
        }
        (None, Some(exception)) => {
            log().exception(exception, None, false, true, true);
            (
                format!("{exception:?}\n\n{exception:#?}"),
                format!("```\n{exception:?}\n```\n\n```{exception:#?}```"),
            )
        }
        (Some(message), None) => {
            log().error(message);
            (message.to_string(), format!("`{message}`"))
        }
        (None, None) => {
            log().error("Unknown error");
            ("Unknown error".to_string(), "`Unknown error`".to_string())
        }
    };

    // Headless fallback so the error is never invisible.
    eprintln!("=== {title} ===\n{full_message}");

    if error {
        rfd::MessageDialog::new()
            .set_title(title)
            .set_description(&full_message)
            .set_level(rfd::MessageLevel::Error)
            .set_buttons(rfd::MessageButtons::Ok)
            .show();

        if ask_yes_no(
            &text(Some("open_github_issue_prompt_title"), None),
            &text(Some("open_github_issue_prompt_message"), None),
        ) {
            let exception_text = exception.map(|e| format!("{e:?}")).unwrap_or_default();
            let url = github_issue_url(title, &exception_text, &github_message);
            crate::app::buttons::system::openfile::openfile(&url);
        } else if ask_yes_no(
            &text(Some("copy_to_clipboard_prompt_title"), None),
            &text(Some("copy_to_clipboard_prompt_message"), None),
        ) {
            match arboard::Clipboard::new()
                .and_then(|mut clipboard| clipboard.set_text(full_message.clone()))
            {
                Ok(()) => {}
                Err(e) => log().error(&format!("clipboard copy failed: {e}")),
            }
        }
    } else {
        // MessageBoxW equivalent: single OK dialog.
        rfd::MessageDialog::new()
            .set_title(title)
            .set_description(&full_message)
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_url_encoding() {
        let url = github_issue_url("WebDeck Error", "boom", "`msg`");
        assert!(url.starts_with("https://github.com/Lenochxd/WebDeck/issues/new?"));
        assert!(url.contains("title=[AUTO]%20WebDeck%20Error%20-%20boom"));
        assert!(url.contains("body="));
        assert!(!url.contains(' '));
        assert!(!url.contains('#'));
    }
}
