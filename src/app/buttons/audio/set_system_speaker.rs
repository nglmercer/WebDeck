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
        match super::policy_config::set_default_by_friendly_name(
            eRender,
            speakers_name,
            &[eConsole],
        ) {
            Ok(true) => log().debug(&format!("Default speakers set to '{speakers_name}'")),
            Ok(false) => log().debug(&format!("Speakers '{speakers_name}' not found")),
            Err(e) => log().exception(&e, Some("Failed to set default speakers"), true, true, true),
        }
    }
    #[cfg(target_os = "linux")]
    {
        // Exact `Description` match (pactl's `FriendlyName` equivalent),
        // switched via `pactl set-default-sink`.
        match super::volume::pactl_output(&["list", "sinks"]) {
            Ok(out) => {
                let found = super::volume::parse_endpoint_names(&out, "Sink #")
                    .into_iter()
                    .find(|(_, description)| description == speakers_name);
                match found {
                    Some((name, _)) => {
                        match super::volume::pactl_run(&["set-default-sink", name.as_str()]) {
                            Ok(()) => {
                                log().debug(&format!("Default speakers set to '{speakers_name}'"))
                            }
                            Err(e) => log().exception(
                                &e,
                                Some("Failed to set default speakers"),
                                true,
                                true,
                                true,
                            ),
                        }
                    }
                    None => log().debug(&format!("Speakers '{speakers_name}' not found")),
                }
            }
            Err(e) => log().exception(&e, Some("Failed to list speakers"), true, true, true),
        }
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        log().warning(&format!(
            "set_speakers_by_name({speakers_name:?}): only supported on Windows and Linux"
        ));
    }
}
