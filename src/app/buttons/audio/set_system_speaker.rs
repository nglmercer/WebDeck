//! Port of `app/buttons/audio/set_system_speaker.py`.
//!
//! Default-speaker switching (exact `FriendlyName` match, pycaw's default
//! Console role), via the shared [`super::policy_config`] helper.

use crate::app::utils::logger::log;

/// Port of `set_speakers_by_name`.
pub fn set_speakers_by_name(speakers_name: &str) {
    #[cfg(windows)]
    {
        use windows::Win32::Media::Audio::{eConsole, eRender};
        match super::policy_config::set_default_by_friendly_name(eRender, speakers_name, &[eConsole])
        {
            Ok(true) => log().debug(&format!("Default speakers set to '{speakers_name}'")),
            Ok(false) => log().debug(&format!("Speakers '{speakers_name}' not found")),
            Err(e) => log().exception(&e, Some("Failed to set default speakers"), true, true, true),
        }
    }
    #[cfg(not(windows))]
    {
        log().warning(&format!(
            "set_speakers_by_name({speakers_name:?}): only supported on Windows"
        ));
    }
}
