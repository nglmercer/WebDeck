//! Boot + home routes (extracted from `server.rs`).
//!
//! `GET /` serves the TypeScript SPA bundle; `GET /api/v2/boot` returns the
//! old Jinja template context as JSON.

use axum::{
    extract::State,
    http::header,
    response::{IntoResponse, Json, Response},
    Extension,
};
use serde_json::{json, Value};

use super::assets::{get_svgs, save_rotated_copy};
use super::{
    internal_error,
    security::{require, Identity},
    AppState,
};
use crate::app::buttons::usage::get_usage;
use crate::app::utils::{
    languages::{get_languages_info, lang_dict},
    logger::log,
    settings::audio_devices::get_audio_devices,
    themes::parse_themes::parse_themes,
};
use crate::domain::command::Capability;

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

/// Boot context for the TypeScript frontend (`GET /api/v2/boot`).
///
/// Read/input controllers receive deck data without integration credentials.
pub(crate) async fn boot(Extension(identity): Extension<Identity>) -> Response {
    match boot_context(require(&identity, Capability::Settings).is_ok()) {
        Ok(context) => Json(context).into_response(),
        Err(message) => internal_error("Cannot load deck", message, None),
    }
}

pub(crate) async fn settings_boot(Extension(identity): Extension<Identity>) -> Response {
    if let Err(error) = require(&identity, Capability::Settings) {
        return super::v2::error_response(error, None);
    }
    match boot_context(true) {
        Ok(context) => Json(context).into_response(),
        Err(message) => internal_error("Cannot load settings", message, None),
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

fn boot_context(privileged: bool) -> Result<Value, String> {
    let service = crate::application::config::shared(
        crate::app::utils::settings::get_config::get_config_path(),
    )
    .map_err(|e| e.message)?;
    let snapshot = service.snapshot().map_err(|e| e.message)?;
    let config = snapshot.config;

    let commands = if privileged {
        crate::application::catalog::snapshot().map_err(|e| e.message)?
    } else {
        json!({})
    };

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
            let rotated = crate::app::utils::settings::get_config::config_dir()
                .join("user_uploads")
                .join(candidate.trim_start_matches("**uploaded/"))
                .to_string_lossy()
                .to_string();
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
    if let Ok(entries) =
        std::fs::read_dir(crate::app::utils::settings::get_config::config_dir().join("themes"))
    {
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
        .and_then(|v| v.as_str())
        .map(str::to_string);
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

    // Use a whitelist: new administrative/integration fields stay private by
    // default. The deck consumes only language and transfer method.
    if !privileged {
        let settings = config["settings"].clone();
        config = json!({
            "schema_version": config["schema_version"],
            "front": config["front"],
            "settings": {
                "language": settings["language"],
                "data_transfer_method": settings["data_transfer_method"],
            },
        });
    }
    Ok(json!({
        "api_version": 2,
        "can_edit": privileged,
        "config_revision": snapshot.revision,
        "config": config,
        "themes": themes,
        "parsed_themes": if privileged { serde_json::to_value(parse_themes()).unwrap_or(Value::Null) } else { json!({}) },
        "commands": if privileged { commands } else { json!({}) },
        "versions": versions,
        "random_bg": random_bg,
        "usage_example": get_usage(Some(true), &[]),
        "langs": serde_json::to_value(get_languages_info()).unwrap_or(Value::Null),
        "svgs": get_svgs(),
        "is_exe": is_exe,
        "portrait_rotate": config.get("front").and_then(|f| f.get("portrait_rotate")).cloned().unwrap_or(Value::Null),
        "lang": serde_json::to_value(lang_dict(configured_lang.as_deref())).unwrap_or(Value::Null),
        "audio_devices": {
            "input": if privileged { get_audio_devices("input") } else { vec![] },
            "output": if privileged { get_audio_devices("output") } else { vec![] },
        },
        "dark_theme": dark_theme,
    }))
}
