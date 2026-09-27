//! Port of `app/buttons/color_picker/notification.py`.
//!
//! Message formatting is ported 1:1 (pure [`format_toast_message`],
//! unit-tested). Delivery is `winrt-notification` on Windows and
//! `notify-rust` on Linux — Python only toasts on Windows, so Linux toasts
//! are a documented improvement (same title/message/duration).

use std::collections::HashMap;

use crate::app::utils::logger::log;

/// Port of the message-building branches in `toast`.
pub fn format_toast_message(
    display_type: Option<&str>,
    typestocopy: Option<&str>,
    color_names_final: &HashMap<String, String>,
) -> String {
    let single_copy =
        typestocopy.map(|t| t.split(';').count() == 1).unwrap_or(false);
    if display_type.map(|d| d.to_lowercase() != "list").unwrap_or(false) {
        if single_copy {
            color_names_final.values().next().cloned().unwrap_or_default()
        } else {
            color_names_final
                .values()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        }
    } else if single_copy {
        // Python: str(dict)[:-2][2:].replace("'", "") — single "K: V" entry.
        color_names_final
            .iter()
            .next()
            .map(|(k, v)| format!("{k}: {v}"))
            .unwrap_or_default()
    } else {
        // Python: str(dict).replace("', ", ",\n")[:-2][2:].replace("'", "").
        color_names_final
            .iter()
            .map(|(k, v)| format!("{k}: {v}"))
            .collect::<Vec<_>>()
            .join(",\n")
    }
}

/// Port of `toast`.
pub fn toast(
    display_type: Option<&str>,
    typestocopy: Option<&str>,
    color_names_final: &HashMap<String, String>,
) {
    const TITLE: &str = "WebDeck Color Picker";
    let message = format_toast_message(display_type, typestocopy, color_names_final);

    #[cfg(windows)]
    {
        let result = winrt_notification::Toast::new(winrt_notification::Toast::POWERSHELL_APP_ID)
            .title(TITLE)
            .text1(&message)
            .duration(winrt_notification::Duration::Short)
            .show();
        if let Err(e) = result {
            log().debug(&format!("toast failed: {e}"));
        }
    }
    #[cfg(not(windows))]
    {
        let result = notify_rust::Notification::new()
            .summary(TITLE)
            .body(&message)
            .icon("static/icons/icon.ico")
            .timeout(notify_rust::Timeout::Milliseconds(5000))
            .show();
        if let Err(e) = result {
            log().debug(&format!("toast failed: {e}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> HashMap<String, String> {
        HashMap::from([
            ("HEX".to_string(), "#ff0000".to_string()),
            ("RGB".to_string(), "rgb(255,0,0)".to_string()),
        ])
    }

    #[test]
    fn raw_display_joins_values() {
        let message = format_toast_message(Some("raw"), Some("hex;rgb"), &sample());
        assert!(message.contains("#ff0000") && message.contains("rgb(255,0,0)"));
    }

    #[test]
    fn single_copy_picks_first_value() {
        let single = HashMap::from([("HEX".to_string(), "#ff0000".to_string())]);
        assert_eq!(
            format_toast_message(Some("raw"), Some("hex"), &single),
            "#ff0000"
        );
    }

    #[test]
    fn list_display_formats_entries() {
        let message = format_toast_message(Some("list"), Some("hex;rgb"), &sample());
        assert!(message.contains("HEX: #ff0000"));
    }
}
