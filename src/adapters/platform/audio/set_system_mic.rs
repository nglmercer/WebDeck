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
            Err(e) => log().exception(
                &e,
                Some("Failed to set default microphone"),
                true,
                true,
                true,
            ),
        }
    }
    #[cfg(target_os = "linux")]
    {
        // Exact `Description` match (pactl's `FriendlyName` equivalent),
        // switched via `pactl set-default-source`.
        match super::volume::pactl_output(&["list", "sources"]) {
            Ok(out) => {
                let found = super::volume::parse_endpoint_names(&out, "Source #")
                    .into_iter()
                    .find(|(_, description)| description == name);
                match found {
                    Some((source, _)) => {
                        match super::volume::pactl_run(&["set-default-source", source.as_str()]) {
                            Ok(()) => log().debug(&format!("Default microphone set to '{name}'")),
                            Err(e) => log().exception(
                                &e,
                                Some("Failed to set default microphone"),
                                true,
                                true,
                                true,
                            ),
                        }
                    }
                    None => log().debug(&format!("Microphone '{name}' not found")),
                }
            }
            Err(e) => log().exception(&e, Some("Failed to list microphones"), true, true, true),
        }
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        log().warning(&format!(
            "set_microphone_by_name({name:?}): only supported on Windows and Linux"
        ));
    }
}
