//! Boot + home routes (extracted from `server.rs`).
//!
//! `GET /` serves the TypeScript SPA bundle; `GET /api/boot` returns the
//! old Jinja template context as JSON.

use axum::{
    extract::State,
    http::header,
    response::{IntoResponse, Json, Response},
};
use serde_json::{json, Value};

use super::assets::{get_svgs, save_rotated_copy};
use super::{internal_error, AppState};
use crate::app::buttons::usage::get_usage;
use crate::app::utils::{
    global_variables::set_global_variable,
    languages::{get_languages_info, lang_dict},
    logger::log,
    plugins::load_plugins::load_plugins,
    settings::{audio_devices::get_audio_devices, get_config::get_config},
    themes::parse_themes::parse_themes,
};

/// Serves the TypeScript frontend (`GET /`).
///
/// The SPA bundle is built with `npm run build` in `frontend/`; without it
/// the route explains how to build instead of serving a broken page.
pub(crate) async fn home(State(_state): State<AppState>) -> Response {
    match std::fs::read("frontend/dist/index.html") {
        Ok(bytes) => ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], bytes).into_response(),
        Err(_) => internal_error(
            "An error occurred during a request",
            "WebDeck frontend is not built. Run `npm run build` in frontend/ and reload."
                .to_string(),
            None,
        ),
    }
}

/// Boot context for the TypeScript frontend (`GET /api/boot`).
///
/// Same data (and side effects: plugin load, wallpaper pick/rotation,
/// style.css pre-push) the Jinja render used to inline into the page.
pub(crate) async fn boot(State(_state): State<AppState>) -> Response {
    match boot_context() {
        Ok(context) => Json(context).into_response(),
        Err(message) => internal_error("An error occurred during a request", message, None),
    }
}

fn pseudo_random_below(len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as usize)
        .unwrap_or(0);
    nanos % len
}

fn boot_context() -> Result<Value, String> {
    let config = get_config(false, true);

    let commands_raw = match std::fs::read_to_string("webdeck/commands.json") {
        Ok(content) => content,
        Err(e) => return Err(format!("Cannot read webdeck/commands.json: {e}")),
    };
    let commands_raw: Value = match serde_json::from_str(&commands_raw) {
        Ok(value) => value,
        Err(e) => return Err(format!("Cannot parse webdeck/commands.json: {e}")),
    };
    let (commands, loaded_plugins) = load_plugins(commands_raw);
    set_global_variable(
        "all_func",
        Value::Array(loaded_plugins.into_iter().map(Value::String).collect()),
    );

    let versions: Value = std::fs::read_to_string("webdeck/version.json")
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or(Value::Null);

    let is_exe = !cfg!(debug_assertions);

    let backgrounds: Vec<String> = config
        .get("front")
        .and_then(|f| f.get("background"))
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let mut random_bg = String::new();
    for _ in 0..100 {
        if backgrounds.is_empty() {
            break;
        }
        let candidate = backgrounds[pseudo_random_below(backgrounds.len())].clone();
        if candidate.starts_with("//") {
            continue;
        }
        if candidate.starts_with("**uploaded/") {
            let rotated = candidate.replace("**uploaded/", ".config/user_uploads/");
            let path = std::path::Path::new(&rotated);
            if path.exists() {
                save_rotated_copy(&rotated);
            }
        }
        random_bg = candidate;
        break;
    }
    log().debug(&format!("Selected random background image: {random_bg}"));

    let mut config = config;
    if let Some(themes) = config
        .get_mut("front")
        .and_then(|f| f.get_mut("themes"))
        .and_then(|t| t.as_array_mut())
    {
        // Mirrors the unconditional `.append("static/css/style.css")` at
        // render time in Flask (fresh config per request, so no accumulation).
        themes.push(Value::String("static/css/style.css".to_string()));
    }

    let mut themes: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(".config/themes/") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".css") {
                themes.push(name);
            }
        }
    }

    let configured_lang = config
        .get("settings")
        .and_then(|s| s.get("language"))
        .and_then(|v| v.as_str());
    // Jinja truthiness (`{% if config['front']['dark_theme'] %}`).
    let dark_theme = match config.get("front").and_then(|f| f.get("dark_theme")) {
        None | Some(Value::Null) | Some(Value::Bool(false)) => "",
        Some(Value::String(s)) if s.is_empty() => "",
        Some(Value::Number(n)) => {
            if n.as_f64().unwrap_or(0.0) == 0.0 {
                ""
            } else {
                " dark-theme"
            }
        }
        Some(Value::Array(a)) if a.is_empty() => "",
        Some(Value::Object(o)) if o.is_empty() => "",
        _ => " dark-theme",
    };

    Ok(json!({
        "config": config,
        "themes": themes,
        "parsed_themes": serde_json::to_value(parse_themes()).unwrap_or(Value::Null),
        "commands": commands,
        "versions": versions,
        "random_bg": random_bg,
        "usage_example": get_usage(Some(true), &[]),
        "langs": serde_json::to_value(get_languages_info()).unwrap_or(Value::Null),
        "svgs": get_svgs(),
        "is_exe": is_exe,
        "portrait_rotate": config.get("front").and_then(|f| f.get("portrait_rotate")).cloned().unwrap_or(Value::Null),
        "lang": serde_json::to_value(lang_dict(configured_lang)).unwrap_or(Value::Null),
        "audio_devices": {
            "input": get_audio_devices("input"),
            "output": get_audio_devices("output"),
        },
        "dark_theme": dark_theme,
    }))
}
