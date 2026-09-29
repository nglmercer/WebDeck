//! Config + button persistence routes (extracted from `server.rs`).
//!
//! `POST /save_config`, `POST /COMPLETE_save_config`,
//! `POST /save_single_button`, `POST /save_buttons_only`,
//! `GET /get_config`, `POST /create_folder`.

use axum::{
    extract::State,
    response::{IntoResponse, Json, Response},
};
use serde_json::{json, Value};

use super::{internal_error, AppState};
use crate::app::buttons::{obs, soundboard};
use crate::app::tray::change_tray_language;
use crate::app::utils::{
    global_variables::set_global_variable,
    languages::set_default_language,
    logger::log,
    merge_dicts::merge_dicts,
    settings::{
        check_config_update::check_config_update, create_folders::create_folders,
        get_config::get_config, gridsize::update_gridsize, save_config::save_config,
    },
};

fn as_usize(value: &Value) -> usize {
    value
        .as_u64()
        .map(|n| n as usize)
        .or_else(|| value.as_str().and_then(|s| s.parse::<usize>().ok()))
        .unwrap_or(0)
}

/// Port of `saveconfig` (`POST /save_config`).
pub(crate) async fn saveconfig(
    State(state): State<AppState>,
    Json(new_config): Json<Value>,
) -> Response {
    let mut config = get_config(false, false);

    let new_height = as_usize(&new_config["front"]["height"]);
    let new_width = as_usize(&new_config["front"]["width"]);
    config = update_gridsize(config, new_height, new_width);
    if let Some(front) = config.get_mut("front").and_then(|f| f.as_object_mut()) {
        front.insert("height".to_string(), json!(new_height));
        front.insert("width".to_string(), json!(new_width));
    }

    let soundboard_restart =
        config["settings"]["soundboard"] != new_config["settings"]["soundboard"];
    let obs_reload = config["settings"]["obs"] != new_config["settings"]["obs"];
    let language_changed = config["settings"]["language"] != new_config["settings"]["language"];

    let (soundboard_start, soundboard_stop) = {
        let old = config["settings"]["soundboard"]["enabled"]
            .as_bool()
            .unwrap_or(false);
        let new = new_config["settings"]["soundboard"]["enabled"]
            .as_bool()
            .unwrap_or(false);
        (new && !old, old && !new)
    };

    let old_startup = config["settings"]["windows_startup"]
        .as_bool()
        .unwrap_or(false);
    let new_startup = new_config["settings"]["windows_startup"]
        .as_bool()
        .unwrap_or(false);
    if !old_startup && new_startup {
        #[cfg(windows)]
        if !cfg!(debug_assertions) {
            crate::app::on_start::utils::create_startup_shortcut();
        }
        #[cfg(target_os = "linux")]
        if !cfg!(debug_assertions) {
            crate::app::on_start::utils::create_startup_shortcut();
        }
    } else if old_startup && !new_startup {
        #[cfg(windows)]
        if !cfg!(debug_assertions) {
            if let Ok(appdata) = std::env::var("APPDATA") {
                let link = format!(
                    "{appdata}\\Microsoft\\Windows\\Start Menu\\Programs\\Startup\\WebDeck.lnk"
                );
                let _ = std::fs::remove_file(link);
            }
        }
        #[cfg(target_os = "linux")]
        if !cfg!(debug_assertions) {
            crate::app::on_start::utils::remove_startup_shortcut();
        }
    }

    config = merge_dicts(config, &new_config);
    let folders: Vec<Value> = {
        let mut queue = state.folders_to_create.lock().await;
        std::mem::take(&mut *queue)
    };
    config = create_folders(config, &folders);
    config = check_config_update(config);
    config = save_config(config);

    // Python re-parses a stringified background list, then saves again.
    if config["front"]["background"].is_string() {
        let raw = config["front"]["background"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let normalized = raw
            .replace("['", "[\"")
            .replace("']", "\"]")
            .replace("', '", "','")
            .replace("','", "\",\"");
        if let Ok(parsed) = serde_json::from_str::<Value>(&normalized) {
            config["front"]["background"] = parsed;
        }
    }
    save_config(config.clone());

    if obs_reload {
        obs::reload_obs();
    }

    if language_changed {
        if let Some(lang) = new_config["settings"]["language"].as_str() {
            set_default_language(lang);
            change_tray_language(lang);
        }
    }

    if soundboard_stop {
        soundboard::mic::stop();
    } else if (soundboard_restart || soundboard_start)
        && config["settings"]["soundboard"]["enabled"]
            .as_bool()
            .unwrap_or(false)
    {
        soundboard::mic::restart();
    }

    log().success("Config saved successfully");
    Json(json!({"success": true})).into_response()
}

/// Port of `complete_save_config` (`POST /COMPLETE_save_config`).
pub(crate) async fn complete_save_config(
    State(state): State<AppState>,
    Json(new_config): Json<Value>,
) -> Response {
    let old = get_config(false, false);
    let old_height = old["front"]["height"].clone();
    let old_width = old["front"]["width"].clone();

    let mut config = new_config;
    let new_height = as_usize(&config["front"]["height"]);
    let new_width = as_usize(&config["front"]["width"]);

    let folders: Vec<Value> = {
        let mut queue = state.folders_to_create.lock().await;
        std::mem::take(&mut *queue)
    };
    config = create_folders(config, &folders);
    config = save_config(config);

    if let Some(front) = config.get_mut("front").and_then(|f| f.as_object_mut()) {
        front.insert("height".to_string(), old_height);
        front.insert("width".to_string(), old_width);
    }
    config = update_gridsize(config, new_height, new_width);
    if let Some(front) = config.get_mut("front").and_then(|f| f.as_object_mut()) {
        front.insert("height".to_string(), json!(new_height));
        front.insert("width".to_string(), json!(new_width));
    }
    save_config(config);

    log().success("Config saved successfully");
    Json(json!({"success": true})).into_response()
}

/// Port of `save_single_button` (`POST /save_single_button`).
pub(crate) async fn save_single_button(Json(data): Json<Value>) -> Response {
    let button_folder = data
        .get("location_Folder")
        .and_then(|v| {
            v.as_u64()
                .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
        })
        .unwrap_or(0) as usize;
    let button_index = data
        .get("location_Id")
        .and_then(|v| {
            v.as_u64()
                .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
        })
        .unwrap_or(0) as usize;
    let button_content = data.get("content").cloned().unwrap_or(Value::Null);

    let mut config = get_config(false, false);
    let folder_name = config
        .get("front")
        .and_then(|f| f.get("buttons"))
        .and_then(|b| b.as_object())
        .and_then(|map| map.keys().nth(button_folder).cloned());

    match folder_name {
        Some(folder_name) => {
            if let Some(slot) = config
                .get_mut("front")
                .and_then(|f| f.get_mut("buttons"))
                .and_then(|b| b.get_mut(&folder_name))
                .and_then(|list| list.as_array_mut())
                .and_then(|list| list.get_mut(button_index))
            {
                log().debug(&format!("FETCH /save_single_button -> before :{slot}"));
                *slot = button_content.clone();
                log().debug(&format!("FETCH /save_single_button -> after  :{slot}"));
            }
            save_config(config);
            log().success("Button saved successfully");
            Json(json!({"success": true})).into_response()
        }
        None => internal_error(
            "An error occurred during a request",
            format!("Button folder index {button_folder} out of range"),
            None,
        ),
    }
}

/// Port of `save_buttons_only` (`POST /save_buttons_only`).
pub(crate) async fn save_buttons_only(
    State(state): State<AppState>,
    Json(new_config): Json<Value>,
) -> Response {
    let mut config = get_config(false, false);

    if let Some(new_buttons) = new_config
        .get("front")
        .and_then(|f| f.get("buttons"))
        .cloned()
    {
        if let Some(buttons) = config.get_mut("front").and_then(|f| f.get_mut("buttons")) {
            *buttons = new_buttons;
        }
    }

    let folders: Vec<Value> = {
        let mut queue = state.folders_to_create.lock().await;
        std::mem::take(&mut *queue)
    };
    let config = create_folders(config, &folders);
    save_config(config);

    log().success("Buttons saved successfully");
    Json(json!({"success": true})).into_response()
}

/// Port of `get_config_route` (`GET /get_config`).
pub(crate) async fn get_config_route(State(state): State<AppState>) -> Response {
    let mut config = get_config(false, false);

    let folders: Vec<Value> = {
        let mut queue = state.folders_to_create.lock().await;
        std::mem::take(&mut *queue)
    };
    config = create_folders(config, &folders);
    set_global_variable("config", config.clone());
    save_config(config.clone());

    Json(config).into_response()
}

/// Port of `create_folder` (`POST /create_folder`).
pub(crate) async fn create_folder(
    State(state): State<AppState>,
    Json(data): Json<Value>,
) -> Response {
    let folder_name = data.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let parent_folder_name = data
        .get("parent_folder")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let config = get_config(false, false);
    let mut queue = state.folders_to_create.lock().await;

    let queued = queue
        .iter()
        .any(|item| item.get("name").and_then(|v| v.as_str()) == Some(folder_name));
    let exists = config
        .get("front")
        .and_then(|f| f.get("buttons"))
        .and_then(|b| b.as_object())
        .map(|map| map.contains_key(folder_name))
        .unwrap_or(false);

    if !queued && !exists {
        queue.push(json!({"name": folder_name, "parent_folder": parent_folder_name}));
        log().info(&format!(
            "Folder '{folder_name}' is in the queue to be created"
        ));
        Json(json!({"success": true})).into_response()
    } else {
        log().error("Folder already exists");
        Json(json!({"success": false, "message": "Folder already exists"})).into_response()
    }
}
