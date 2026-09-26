//! Port of `app/buttons/color_picker/notification.py`.
//!
//! TODO(port): toast via `winrt-notification` (Windows) / `notify-rust`
//! (Linux), replacing `win10toast`.

use std::collections::HashMap;

use crate::app::utils::logger::log;

/// Port of `toast` — stub that logs what the toast would show.
pub fn toast(
    display_type: Option<&str>,
    typestocopy: Option<&str>,
    color_names_final: &HashMap<String, String>,
) {
    log().info(&format!(
        "toast({display_type:?}, {typestocopy:?}): {color_names_final:?} (toast backend not ported yet)"
    ));
}
