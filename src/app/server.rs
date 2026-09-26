//! Port of `app/server.py` — Flask → axum.
//!
//! Route table (same paths + methods as the Flask app):
//! `GET /`, `POST /usage`, `POST /save_config`, `POST /COMPLETE_save_config`,
//! `POST /save_single_button`, `POST /save_buttons_only`, `GET /get_config`,
//! `POST /upload_folderpath`, `POST /upload_filepath`, `POST /upload_file`,
//! `POST /create_folder`, `GET /.config/<dir>/<file>`, `POST /send-data`,
//! plus `/static/*` (Flask static folder).
//!
//! Mapping notes:
//! - `@app.before_request check_local_network` → [`check_local_network`]
//!   middleware. One intentional deviation: Python tests
//!   `remote_ip in ipaddress.ip_address(network)` for `allowed_networks`,
//!   which raises `TypeError` (an `Address` is not a container); Rust
//!   implements the evidently intended CIDR-contains semantics.
//! - `@app.after_request` → [`after_request`] middleware (skips `/usage`).
//! - `@app.errorhandler(Exception)` → [`internal_error`] (always JSON; the
//!   `flask_debug` HTML-fallthrough is a dev-only path and is not mirrored).
//! - `render_template("index.jinja")` → minijinja with the same context keys.
//!   Template helpers `int/str/dict/type/eval/open` are Python builtins with
//!   no sandbox-safe equivalent and are NOT exposed; templates using them
//!   need the follow-up "template sandbox adaptation" (see
//!   `docs/MIGRATION_RUST.md`). `text/get_language/isfile/get_audio_devices/
//!   mdebug` are exposed as minijinja functions.
//! - Flask-SocketIO (`connect`/`send`/`message_from_socket`) is TODO via
//!   `socketioxide` — the emit points are marked in `handle_command` callers.
//! - `werkzeug` vs `app.run` selection collapses to one axum backend.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{ConnectInfo, DefaultBodyLimit, Multipart, Path, Query, State},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use minijinja::value::Rest;
use serde_json::{json, Value};
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

use crate::app::buttons::{self, soundboard, usage::get_usage};
use crate::app::on_start::on_start;
use crate::app::tray::{change_server_state, change_tray_language};
use crate::app::utils::{
    args::get_args,
    firewall::{check_firewall_permission, fix_firewall_permission},
    global_variables::set_global_variable,
    languages::{get_language, get_languages_info, set_default_language, text},
    logger::log,
    merge_dicts::merge_dicts,
    plugins::load_plugins::load_plugins,
    settings::{
        audio_devices::get_audio_devices,
        check_config_update::check_config_update,
        create_folders::create_folders,
        get_config::{get_config, get_port},
        gridsize::update_gridsize,
        save_config::save_config,
    },
    themes::parse_themes::parse_themes,
};

/// Shared server state — port of the module-level `config`,
/// `folders_to_create`, and `local_ip` globals in `app/server.py`.
/// (Config itself is re-read from disk per handler, exactly like Python.)
#[derive(Clone)]
pub struct AppState {
    pub folders_to_create: Arc<Mutex<Vec<Value>>>,
    pub local_ip: String,
}

/// Server startup/serve error (port of exceptions out of `run_server`).
#[derive(Debug)]
pub struct ServerError(pub String);

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Server error: {}", self.0)
    }
}

fn as_usize(value: &Value) -> usize {
    value
        .as_u64()
        .map(|n| n as usize)
        .or_else(|| value.as_str().and_then(|s| s.parse::<usize>().ok()))
        .unwrap_or(0)
}

/// Port of the Flask `@app.errorhandler(Exception)` — logs like
/// `handle_exception` and returns the 500 JSON.
fn internal_error(context: &str, detail: String, req: Option<(&str, &str, &str)>) -> Response {
    log().exception(&detail, Some(context), true, true, true);
    if let Some((remote, method, url)) = req {
        log().httprequest(remote, method, url, 500);
    }
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"success": false, "message": detail})),
    )
        .into_response()
}

// --- Middleware --------------------------------------------------------

/// Port of `@app.before_request check_local_network`.
async fn check_local_network(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let config = get_config(false, false);
    let netmask: u8 = config
        .get("settings")
        .and_then(|s| s.get("netmask"))
        .and_then(|v| v.as_u64())
        .unwrap_or(16) as u8;

    // Requests without peer info (e.g. tests) are allowed through.
    if let Some(peer) = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip())
    {
        if !ip_allowed(peer, &state.local_ip, netmask, &config) {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({"success": false, "message": "Access denied: IP not in local network"})),
            )
                .into_response();
        }
    }

    next.run(req).await
}

fn ipv4_masked(ip: Ipv4Addr, prefix: u8) -> u32 {
    let bits = u32::from(ip);
    if prefix >= 32 {
        bits
    } else {
        bits & (!0u32 << (32 - prefix))
    }
}

/// Same-network check + `allowed_networks` (CIDR or single IP entries).
fn ip_allowed(remote: IpAddr, local_ip: &str, netmask: u8, config: &Value) -> bool {
    // Non-IPv4 remotes or unparseable local IP: allow (the app is
    // IPv4-oriented; loopback/v6 stays reachable in dev).
    let IpAddr::V4(remote_v4) = remote else {
        return true;
    };
    let Ok(local) = local_ip.parse::<Ipv4Addr>() else {
        return true;
    };

    if ipv4_masked(remote_v4, netmask) == ipv4_masked(local, netmask) {
        return true;
    }
    if let Some(networks) = config
        .get("settings")
        .and_then(|s| s.get("allowed_networks"))
        .and_then(|v| v.as_array())
    {
        for network in networks.iter().filter_map(|v| v.as_str()) {
            if let Some((base, prefix)) = network.split_once('/') {
                if let (Ok(base), Ok(prefix)) = (base.parse::<Ipv4Addr>(), prefix.parse::<u8>())
                {
                    if ipv4_masked(remote_v4, prefix) == ipv4_masked(base, prefix) {
                        return true;
                    }
                }
            } else if network.parse::<IpAddr>() == Ok(remote) {
                return true;
            }
        }
    }
    false
}

/// Port of `@app.after_request` (skips `/usage`, like Python).
async fn after_request(req: Request<Body>, next: Next) -> Response {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let url = req.uri().to_string();
    let remote = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.to_string())
        .unwrap_or_else(|| "-".to_string());
    let response = next.run(req).await;
    if path != "/usage" {
        log().httprequest(&remote, &method, &url, response.status().as_u16());
    }
    response
}

// --- Helpers -----------------------------------------------------------

/// Port of `get_svgs` — collects `url(….svg)` references from style.css.
pub fn get_svgs() -> Vec<String> {
    let mut svgs = Vec::new();
    if let Ok(content) = std::fs::read_to_string("static/css/style.css") {
        let mut rest = content.as_str();
        while let Some(start) = rest.find("url(") {
            rest = &rest[start + 4..];
            let Some(end) = rest.find(')') else {
                break;
            };
            let reference = rest[..end].trim().trim_matches(|c| c == '"' || c == '\'');
            if reference.ends_with(".svg") {
                svgs.push(reference.to_string());
            }
            rest = &rest[end + 1..];
        }
    }
    svgs
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

fn minijinja_env() -> minijinja::Environment<'static> {
    let mut env = minijinja::Environment::new();
    env.set_loader(minijinja::path_loader("templates"));
    env.add_function("get_audio_devices", |args: Rest<String>| {
        let channels = args.first().map(|s| s.as_str()).unwrap_or("input");
        minijinja::Value::from_serialize(&get_audio_devices(channels))
    });
    env.add_function("mdebug", |message: minijinja::Value| {
        log().debug(&format!("{message:?}"));
        String::new()
    });
    env.add_function("text", |args: Rest<String>| {
        let key = args.first().cloned().unwrap_or_default();
        let lang = args.get(1).cloned();
        text(Some(&key), lang.as_deref())
    });
    env.add_function("get_language", |args: Rest<String>| {
        get_language(args.first().map(|s| s.as_str()))
    });
    env.add_function("isfile", |path: String| {
        std::path::Path::new(&path).is_file()
    });
    env
}

// --- Routes ------------------------------------------------------------

/// Port of `usage` (`POST /usage`).
async fn usage() -> Json<Value> {
    Json(get_usage(None, &[]))
}

/// Port of `home` (`GET /`).
async fn home(State(_state): State<AppState>) -> Response {
    let config = get_config(false, true);

    let commands_raw = match std::fs::read_to_string("webdeck/commands.json") {
        Ok(content) => content,
        Err(e) => {
            return internal_error(
                "An error occurred during a request",
                format!("Cannot read webdeck/commands.json: {e}"),
                None,
            );
        }
    };
    let commands_raw: Value = match serde_json::from_str(&commands_raw) {
        Ok(value) => value,
        Err(e) => {
            return internal_error(
                "An error occurred during a request",
                format!("Cannot parse webdeck/commands.json: {e}"),
                None,
            );
        }
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
            let rotated = candidate.replace(
                "**uploaded/",
                ".config/user_uploads/",
            );
            let path = std::path::Path::new(&rotated);
            if path.exists() {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                    let rotated_path = format!(".config/user_uploads/{stem}-90.{ext}");
                    if !std::path::Path::new(&rotated_path).exists() {
                        // TODO(port): image rotation via the `image` crate.
                        log().debug(&format!(
                            "Portrait rotation for {rotated} deferred (image crate TODO)"
                        ));
                    }
                }
            }
        }
        random_bg = candidate;
        break;
    }
    log().debug(&format!("Selected random background image: {random_bg}"));

    let mut themes: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(".config/themes/") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".css") {
                themes.push(name);
            }
        }
    }

    let context = json!({
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
    });

    let env = minijinja_env();
    match env.get_template("index.jinja") {
        Ok(template) => {
            // A template bug must never kill the worker connection: Jinja2
            // tolerates expressions minijinja cannot (e.g. `[::-1]` on an
            // empty list currently panics inside minijinja instead of
            // returning empty). Contain that as a 500 until the templates
            // get their sandbox-adaptation pass (see docs/MIGRATION_RUST.md).
            let rendered = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                template.render(&context)
            }));
            match rendered {
                Ok(Ok(html)) => Html(html).into_response(),
                Ok(Err(e)) => internal_error(
                    "An error occurred during a request",
                    format!("Template render failed: {e}"),
                    None,
                ),
                Err(_) => internal_error(
                    "An error occurred during a request",
                    "Template render panicked (minijinja/Jinja2 behavior gap — see docs/MIGRATION_RUST.md)"
                        .to_string(),
                    None,
                ),
            }
        }
        Err(e) => internal_error(
            "An error occurred during a request",
            format!("Cannot load index.jinja: {e}"),
            None,
        ),
    }
}

/// Port of `saveconfig` (`POST /save_config`).
async fn saveconfig(State(state): State<AppState>, Json(new_config): Json<Value>) -> Response {
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
    let language_changed =
        config["settings"]["language"] != new_config["settings"]["language"];

    let (soundboard_start, soundboard_stop) = {
        let old = config["settings"]["soundboard"]["enabled"]
            .as_bool()
            .unwrap_or(false);
        let new = new_config["settings"]["soundboard"]["enabled"]
            .as_bool()
            .unwrap_or(false);
        (new && !old, old && !new)
    };

    let old_startup = config["settings"]["windows_startup"].as_bool().unwrap_or(false);
    let new_startup = new_config["settings"]["windows_startup"].as_bool().unwrap_or(false);
    if !old_startup && new_startup {
        #[cfg(windows)]
        if !cfg!(debug_assertions) {
            // TODO(port): Startup shortcut via windows crate (WScript.Shell).
            log().debug("windows_startup shortcut creation not ported yet");
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
        let raw = config["front"]["background"].as_str().unwrap_or("").to_string();
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
        buttons::obs::reload_obs();
    }

    if language_changed {
        if let Some(lang) = new_config["settings"]["language"].as_str() {
            set_default_language(lang);
            change_tray_language(lang);
        }
    }

    if soundboard_stop {
        soundboard::mic::stop();
    } else if soundboard_restart || soundboard_start {
        if config["settings"]["soundboard"]["enabled"].as_bool().unwrap_or(false) {
            soundboard::mic::restart();
        }
    }

    log().success("Config saved successfully");
    Json(json!({"success": true})).into_response()
}

/// Port of `complete_save_config` (`POST /COMPLETE_save_config`).
async fn complete_save_config(
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
async fn save_single_button(Json(data): Json<Value>) -> Response {
    let button_folder = data.get("location_Folder").and_then(|v| {
        v.as_u64().or_else(|| {
            v.as_str().and_then(|s| s.parse::<u64>().ok())
        })
    }).unwrap_or(0) as usize;
    let button_index = data.get("location_Id").and_then(|v| {
        v.as_u64().or_else(|| {
            v.as_str().and_then(|s| s.parse::<u64>().ok())
        })
    }).unwrap_or(0) as usize;
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
async fn save_buttons_only(
    State(state): State<AppState>,
    Json(new_config): Json<Value>,
) -> Response {
    let mut config = get_config(false, false);

    if let Some(new_buttons) = new_config.get("front").and_then(|f| f.get("buttons")).cloned() {
        if let Some(buttons) = config
            .get_mut("front")
            .and_then(|f| f.get_mut("buttons"))
        {
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
async fn get_config_route(State(state): State<AppState>) -> Response {
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

/// Port of `upload_folderpath` (`POST /upload_folderpath`).
/// TODO(port): native dialog via `rfd` (easygui equivalent).
async fn upload_folderpath() -> String {
    log().debug("upload_folderpath: native dialog not ported yet (rfd planned)");
    String::new()
}

/// Port of `upload_filepath` (`POST /upload_filepath`).
/// TODO(port): native dialog via `rfd` (easygui equivalent).
async fn upload_filepath(Query(params): Query<HashMap<String, String>>) -> String {
    let filetypes = params.get("filetypes").map(|s| s.as_str());
    log().debug(&format!(
        "upload_filepath(filetypes={filetypes:?}): native dialog not ported yet (rfd planned)"
    ));
    let _ = filetypes.map(|f| {
        f.split('_')
            .map(|item| format!("*{item}"))
            .collect::<Vec<_>>()
    });
    String::new()
}

/// Port of `upload_file` (`POST /upload_file`).
async fn upload_file(mut multipart: Multipart) -> Response {
    let mut saved_name: Option<String> = None;
    let mut info: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();
        if field_name == "file" {
            let filename = field.file_name().unwrap_or("upload.bin").to_string();
            let filename = std::path::Path::new(&filename)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("upload.bin")
                .to_string();
            match field.bytes().await {
                Ok(bytes) => {
                    let save_path = format!(".config/user_uploads/{filename}");
                    match std::fs::write(&save_path, &bytes) {
                        Ok(()) => saved_name = Some(filename),
                        Err(e) => {
                            return internal_error(
                                "An error occurred during a request",
                                format!("Cannot save upload: {e}"),
                                None,
                            );
                        }
                    }
                }
                Err(e) => {
                    return internal_error(
                        "An error occurred during a request",
                        format!("Cannot read upload: {e}"),
                        None,
                    );
                }
            }
        } else if field_name == "info" {
            info = field.text().await.ok();
        }
    }

    let Some(filename) = saved_name else {
        log().error("No files were found in the request.");
        return Json(json!({"success": false, "message": text(Some("no_files_found_error"), None)}))
            .into_response();
    };

    if info.as_deref() == Some("background_image") {
        // TODO(port): -90° rotation via the `image` crate.
        log().debug(&format!(
            "Portrait rotation for uploaded {filename} deferred (image crate TODO)"
        ));
    }

    log().success(&format!("File '{filename}' uploaded successfully"));
    Json(json!({"success": true, "message": text(Some("downloaded_successfully"), None)}))
        .into_response()
}

/// Port of `create_folder` (`POST /create_folder`).
async fn create_folder(State(state): State<AppState>, Json(data): Json<Value>) -> Response {
    let folder_name = data.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let parent_folder_name = data.get("parent_folder").and_then(|v| v.as_str()).unwrap_or("");

    let config = get_config(false, false);
    let mut queue = state.folders_to_create.lock().await;

    let queued = queue.iter().any(|item| {
        item.get("name").and_then(|v| v.as_str()) == Some(folder_name)
    });
    let exists = config
        .get("front")
        .and_then(|f| f.get("buttons"))
        .and_then(|b| b.as_object())
        .map(|map| map.contains_key(folder_name))
        .unwrap_or(false);

    if !queued && !exists {
        queue.push(json!({"name": folder_name, "parent_folder": parent_folder_name}));
        log().info(&format!("Folder '{folder_name}' is in the queue to be created"));
        Json(json!({"success": true})).into_response()
    } else {
        log().error("Folder already exists");
        Json(json!({"success": false, "message": "Folder already exists"})).into_response()
    }
}

/// Port of `get_config_file` (`GET /.config/<directory>/<filename>`).
async fn get_config_file(Path((directory, filename)): Path<(String, String)>) -> Response {
    if directory != "user_uploads" && directory != "themes" {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    let filename = std::path::Path::new(&filename)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let file_path = format!(".config/{directory}/{filename}");

    match tokio::fs::read(&file_path).await {
        Ok(bytes) => match Response::builder()
            .header("content-type", "application/octet-stream")
            .header(
                "content-disposition",
                format!("attachment; filename=\"{filename}\""),
            )
            .body(Body::from(bytes))
        {
            Ok(response) => response.into_response(),
            Err(e) => internal_error(
                "An error occurred during a request",
                format!("Error: {e}"),
                None,
            ),
        },
        Err(_) => (
            StatusCode::NOT_FOUND,
            format!("File '{filename}' not found."),
        )
            .into_response(),
    }
}

/// Port of `send_data_route` (`POST /send-data`).
/// (The SocketIO `message_from_socket` twin is TODO with `socketioxide`.)
async fn send_data_route(Json(body): Json<Value>) -> Response {
    let message = body.get("message").and_then(|v| v.as_str()).unwrap_or("");
    let result = buttons::handle_command(message);
    // TODO(socketioxide): also emit `json_data` to SocketIO clients.
    Json(result).into_response()
}

/// Port of `run_server`.
///
/// Python runs `on_start()` + firewall checks at import time; Rust runs them
/// at the top of this function (same order, same conditions).
pub async fn run_server() -> Result<(), ServerError> {
    let (config, _commands, local_ip) = on_start().await;
    set_global_variable("config", config.clone());

    // Python module level: firewall bypass + local IP log.
    if config["settings"]["automatic_firewall_bypass"].as_bool() == Some(true)
        && !check_firewall_permission()
    {
        fix_firewall_permission();
    }
    log().info(&format!("Local IP address detected: {local_ip}"));

    change_server_state(1);

    let state = AppState {
        folders_to_create: Arc::new(Mutex::new(Vec::new())),
        local_ip: local_ip.clone(),
    };

    let app = Router::new()
        .route("/", get(home))
        .route("/usage", post(usage))
        .route("/save_config", post(saveconfig))
        .route("/COMPLETE_save_config", post(complete_save_config))
        .route("/save_single_button", post(save_single_button))
        .route("/save_buttons_only", post(save_buttons_only))
        .route("/get_config", get(get_config_route))
        .route("/upload_folderpath", post(upload_folderpath))
        .route("/upload_filepath", post(upload_filepath))
        .route("/upload_file", post(upload_file))
        .route("/create_folder", post(create_folder))
        .route("/.config/{directory}/{filename}", get(get_config_file))
        .route("/send-data", post(send_data_route))
        .nest_service("/static", ServeDir::new("static"))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            check_local_network,
        ))
        .layer(DefaultBodyLimit::disable())
        .with_state(state)
        .layer(middleware::from_fn(after_request));

    // TODO(socketioxide): mount SocketIO layer (`connect` log, `send` relay,
    // `message_from_socket` → handle_command → emit `json_data`).

    let host = get_args().host.clone().unwrap_or(local_ip);
    let port = get_port();
    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}"))
        .await
        .map_err(|e| ServerError(format!("Cannot bind {host}:{port}: {e}")))?;
    log().info(&format!("Serving on {host}:{port}"));
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .map_err(|e| ServerError(format!("Server failed: {e}")))?;
    Ok(())
}
