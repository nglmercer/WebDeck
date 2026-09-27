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

    // Custom buttons report back their label; treat anything that is not an
    // explicit "don't show again" as a plain dismiss.
    if result.to_string() == dont_show {
        log().info("Disabling popup message");
        if let Some(settings) = config.get_mut("settings").and_then(|s| s.as_object_mut()) {
            settings.insert("show_popup".to_string(), serde_json::Value::Bool(false));
        }
        save_config(config);
    }
}
