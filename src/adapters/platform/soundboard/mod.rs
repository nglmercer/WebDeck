//! Port of `app/buttons/soundboard/`.

pub mod devices;
pub mod ffmpeg;
pub mod mic;
pub mod player;

pub use player::{
    get_params, playsound, playsound_nava, playsound_vlc, remove_player, stopsound, SoundParams,
};
