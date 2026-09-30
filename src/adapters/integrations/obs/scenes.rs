//! Port of `app/buttons/obs/scenes.py`.

use crate::adapters::integrations::obs::utils::block_on;
use crate::app::utils::logger::log;

/// Port of `set`: switch to the scene whose name matches (case-insensitive).
pub fn set(client: &obws::Client, scene_name: &str) -> Result<(), String> {
    let scenes = match block_on(client.scenes().list()) {
        Ok(scenes) => scenes,
        Err(e) => return Err(e.to_string()),
    };
    for scene in &scenes.scenes {
        if scene.id.name.to_lowercase().trim() == scene_name.to_lowercase().trim() {
            // Python ignores the SetCurrentProgramScene result; mirrored 1:1.
            let _ = block_on(
                client
                    .scenes()
                    .set_current_program_scene(scene.id.name.as_str()),
            );
            log().success(&format!("Switched to scene '{}'", scene.id.name));
            return Ok(());
        }
    }
    log().warning(&format!("Scene '{scene_name}' not found."));
    Err(format!("Scene '{scene_name}' not found."))
}
