//! Port of `app/buttons/audio/set_system_mic.py`.
//!
//! Default-microphone switching for the Communications role (exact
//! `FriendlyName` match), via the shared [`super::policy_config`] helper.

use crate::app::utils::logger::log;

/// Port of `set_microphone_by_name`.
pub fn set_microphone_by_name(name: &str) {
    #[cfg(windows)]
    {
        use windows::Win32::Media::Audio::{eCapture, eCommunications};
        match super::policy_config::set_default_by_friendly_name(eCapture, name, &[eCommunications])
        {
            Ok(true) => log().debug(&format!("Default microphone set to '{name}'")),
            Ok(false) => log().debug(&format!("Microphone '{name}' not found")),
            Err(e) => log().exception(&e, Some("Failed to set default microphone"), true, true, true),
        }
    }
    #[cfg(not(windows))]
    {
        log().warning(&format!(
            "set_microphone_by_name({name:?}): only supported on Windows"
        ));
    }
}
