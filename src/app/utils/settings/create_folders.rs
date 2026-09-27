//! Port of `app/utils/settings/create_folders.py`.

use serde_json::{json, Value};

use crate::app::utils::logger::log;

/// Port of `create_folders`.
///
/// Python also rebinds its local `folders_to_create` to `[]` (a no-op for the
/// caller — server.py clears its own global); callers here clear their own
/// queue the same way.
pub fn create_folders(mut config: Value, folders_to_create: &[Value]) -> Value {
    for folder in folders_to_create {
        let name = folder.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let parent = folder
            .get("parent_folder")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let mut buttons = vec![json!({
            "image": "back10.svg",
            "image_size": "110%",
            "message": format!("/folder {parent}"),
            "name": format!("back to {parent}"),
        })];

        let width = config
            .get("front")
            .and_then(|f| f.get("width"))
            .and_then(|v| {
                v.as_str()
                    .and_then(|s| s.parse::<usize>().ok())
                    .or_else(|| v.as_u64().map(|n| n as usize))
            })
            .unwrap_or(0);
        let height = config
            .get("front")
            .and_then(|f| f.get("height"))
            .and_then(|v| {
                v.as_str()
                    .and_then(|s| s.parse::<usize>().ok())
                    .or_else(|| v.as_u64().map(|n| n as usize))
            })
            .unwrap_or(0);
        let void_count = width.saturating_mul(height);
        for _ in 1..void_count {
            buttons.push(json!({"VOID": "VOID"}));
        }

        if let Some(map) = config
            .get_mut("front")
            .and_then(|f| f.get_mut("buttons"))
            .and_then(|b| b.as_object_mut())
        {
            map.insert(name.to_string(), Value::Array(buttons));
        }

        log().info(&format!("Creating new folder: {name}"));
    }
    config
}
