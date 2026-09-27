//! Port of `app/utils/welcome_popup.py`.
//!
//! Same `settings.show_popup` gate and same texts/buttons; the customtkinter
//! window maps to an `rfd` native dialog (logo image aside — native dialogs
//! have no image slot). Button order and labels mirror Python: "don't show
//! again" disables the popup via config save, "OK" just closes.
//!
//! NOTE: blocks on the native dialog — call off the async runtime
//! (it already runs on a `spawn_blocking` thread from `main`).

use crate::app::utils::{
    languages::text, logger::log, settings::get_config::get_config,
    settings::save_config::save_config,
};

/// True when the dialog result is an explicit "don't show again".
///
/// NOTE: compare the enum, not `to_string()` — rfd renders
/// `Custom(label)` as `"Custom(label)"`, so a string comparison against the
/// bare label never matches and the popup can never be disabled.
fn is_dont_show_again(result: &rfd::MessageDialogResult, dont_show_label: &str) -> bool {
    matches!(result, rfd::MessageDialogResult::Custom(label) if label == dont_show_label)
}

/// Port of `show_popup`.
pub fn show_popup() {
    let mut config = get_config(false, false);
    let show = config
        .get("settings")
        .and_then(|s| s.get("show_popup"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if !show {
        return;
    }

    let dont_show = text(Some("welcome_message_button_dont_show_again"), None);
    let ok = text(Some("welcome_message_button_ok"), None);
    let result = rfd::MessageDialog::new()
        .set_title(&text(Some("welcome_message_window_title"), None))
        .set_description(&format!(
            "{}\n{}",
            text(Some("welcome_message_label_1"), None),
            text(Some("welcome_message_label_2"), None),
        ))
        .set_buttons(rfd::MessageButtons::OkCancelCustom(dont_show.clone(), ok))
        .show();

    // Treat anything that is not an explicit "don't show again" as a
    // plain dismiss.
    if is_dont_show_again(&result, &dont_show) {
        log().info("Disabling popup message");
        if let Some(settings) = config.get_mut("settings").and_then(|s| s.as_object_mut()) {
            settings.insert("show_popup".to_string(), serde_json::Value::Bool(false));
        }
        save_config(config);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_dont_show_again_result() {
        let label = "No mostrar de nuevo";
        assert!(is_dont_show_again(
            &rfd::MessageDialogResult::Custom(label.to_string()),
            label
        ));
    }

    #[test]
    fn treats_other_results_as_plain_dismiss() {
        let label = "No mostrar de nuevo";
        for result in [
            rfd::MessageDialogResult::Ok,
            rfd::MessageDialogResult::Cancel,
            rfd::MessageDialogResult::Yes,
            rfd::MessageDialogResult::No,
            rfd::MessageDialogResult::Custom("Aceptar".to_string()),
        ] {
            assert!(!is_dont_show_again(&result, label), "result: {result:?}");
        }
    }
}
