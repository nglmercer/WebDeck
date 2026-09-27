//! Port of `app/buttons/audio/`.

#[cfg(windows)]
pub mod policy_config;
pub mod set_system_mic;
pub mod set_system_speaker;
pub mod volume;

pub use set_system_mic::set_microphone_by_name;
pub use set_system_speaker::set_speakers_by_name;
pub use volume::handle_command as change_volume;
