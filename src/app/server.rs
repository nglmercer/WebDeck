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
use minijinja::value::{ObjectRepr, Rest};
use serde_json::{json, Value};
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

use crate::app::buttons::{self, soundboard, usage::get_usage};
use crate::app::on_start::on_start;
use crate::app::tray::{change_server_state, change_tray_language, ServerState};
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
                Json(
                    json!({"success": false, "message": "Access denied: IP not in local network"}),
                ),
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
                if let (Ok(base), Ok(prefix)) = (base.parse::<Ipv4Addr>(), prefix.parse::<u8>()) {
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

/// Port of the `img.rotate(-90, expand=True)` blocks: saves a `-90` portrait
/// copy next to the original unless it already exists.
fn save_rotated_copy(original: &str) {
    let path = std::path::Path::new(original);
    let (Some(stem), ext) = (
        path.file_stem().and_then(|s| s.to_str()),
        path.extension().and_then(|s| s.to_str()).unwrap_or(""),
    ) else {
        return;
    };
    let parent = path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let rotated_path = if parent.is_empty() {
        format!("{stem}-90.{ext}")
    } else {
        format!("{parent}/{stem}-90.{ext}")
    };
    if std::path::Path::new(&rotated_path).exists() {
        return;
    }
    match image::open(original) {
        Ok(img) => {
            // PIL rotate(-90) = 90° clockwise = rotate270.
            if let Err(e) = img.rotate270().save(&rotated_path) {
                log().exception(
                    &e,
                    Some(&format!("Failed to rotate image {original}")),
                    true,
                    true,
                    true,
                );
            }
        }
        Err(e) => {
            log().exception(
                &e,
                Some(&format!("Failed to rotate image {original}")),
                true,
                true,
                true,
            );
        }
    }
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
    // Python builtins the templates use as globals.
    env.add_function(
        "int",
        |value: minijinja::Value| -> Result<i64, minijinja::Error> {
            py_int(&value).ok_or_else(|| {
                minijinja::Error::new(
                    minijinja::ErrorKind::InvalidOperation,
                    "int() argument must be a number or numeric string",
                )
            })
        },
    );
    env.add_function("str", |value: minijinja::Value| py_str(&value));
    // `open(path)` reads a text file for inlining (SVG contents); `.read()`
    // on the result is emulated in `python_method` so `open(p).read()`
    // renders the file exactly like Flask.
    env.add_function("open", |path: String| -> Result<String, minijinja::Error> {
        std::fs::read_to_string(&path).map_err(|e| {
            minijinja::Error::new(
                minijinja::ErrorKind::InvalidOperation,
                format!("open({path:?}) failed: {e}"),
            )
        })
    });
    env.add_function(
        "eval",
        |source: String| -> Result<minijinja::Value, minijinja::Error> {
            eval_literal(&source).ok_or_else(|| {
                minijinja::Error::new(
                    minijinja::ErrorKind::InvalidOperation,
                    format!("eval() cannot parse {source:?}"),
                )
            })
        },
    );
    // Python-style `value.method(...)` calls minijinja doesn't implement.
    env.set_unknown_method_callback(python_method);
    // Jinja2's `length` returns 0 for Undefined (its `__len__`); minijinja
    // errors instead, so override with Undefined-tolerant behavior.
    env.add_filter("length", jinja_length);
    env
}

/// Port of Jinja2's `length` filter (`Undefined` → 0, like `Undefined.__len__`).
fn jinja_length(value: minijinja::Value) -> Result<usize, minijinja::Error> {
    if value.is_undefined() {
        return Ok(0);
    }
    if let Some(s) = value.as_str() {
        return Ok(s.chars().count());
    }
    if is_map_value(&value) {
        return value
            .as_object()
            .and_then(|object| object.try_iter_pairs())
            .map(|pairs| pairs.count())
            .ok_or_else(|| {
                minijinja::Error::new(
                    minijinja::ErrorKind::InvalidOperation,
                    "cannot calculate length of value",
                )
            });
    }
    value.try_iter().map(|iter| iter.count()).map_err(|_| {
        minijinja::Error::new(
            minijinja::ErrorKind::InvalidOperation,
            "cannot calculate length of value",
        )
    })
}

/// Emulate the Python str/list/dict methods the templates call
/// (`split`, `startswith`, `join`, `get`, `items`, …).
fn python_method(
    _state: &minijinja::State,
    value: &minijinja::Value,
    method: &str,
    args: &[minijinja::Value],
) -> Result<minijinja::Value, minijinja::Error> {
    use minijinja::{Error, ErrorKind, Value};
    let unsupported = || {
        Error::new(
            ErrorKind::InvalidOperation,
            format!("unsupported method {method}"),
        )
    };
    let arg_str = |i: usize| -> Result<&str, Error> {
        args.get(i).and_then(|v| v.as_str()).ok_or_else(unsupported)
    };

    // Lenient-Undefined chaining (Jinja2 default): any method on Undefined
    // yields Undefined instead of erroring.
    if value.is_undefined() {
        return Ok(Value::UNDEFINED);
    }

    if let Some(s) = value.as_str() {
        let owned = s.to_string();
        return match method {
            "lower" => Ok(Value::from(owned.to_lowercase())),
            "upper" => Ok(Value::from(owned.to_uppercase())),
            "capitalize" => {
                let mut chars = owned.chars();
                let out = match chars.next() {
                    Some(first) => {
                        let mut out: String = first.to_uppercase().collect();
                        out.push_str(&chars.as_str().to_lowercase());
                        out
                    }
                    None => String::new(),
                };
                Ok(Value::from(out))
            }
            "title" => Ok(Value::from(
                owned
                    .split_whitespace()
                    .map(|word| {
                        let mut chars = word.chars();
                        match chars.next() {
                            Some(first) => {
                                first.to_uppercase().collect::<String>() + chars.as_str()
                            }
                            None => String::new(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" "),
            )),
            "strip" => Ok(Value::from(owned.trim().to_string())),
            "lstrip" => Ok(Value::from(owned.trim_start().to_string())),
            "rstrip" => Ok(Value::from(owned.trim_end().to_string())),
            "split" => {
                let parts: Vec<Value> = match args.first().and_then(|v| v.as_str()) {
                    Some(sep) => owned.split(sep).map(Value::from).collect(),
                    None => owned.split_whitespace().map(Value::from).collect(),
                };
                Ok(Value::from(parts))
            }
            "rsplit" => {
                let parts: Vec<Value> = match args.first().and_then(|v| v.as_str()) {
                    Some(sep) => owned.rsplit(sep).map(Value::from).collect(),
                    None => owned.split_whitespace().map(Value::from).collect(),
                };
                Ok(Value::from(parts))
            }
            "splitlines" => Ok(Value::from(
                owned.lines().map(Value::from).collect::<Vec<_>>(),
            )),
            "replace" => {
                let from = arg_str(0)?;
                let to = arg_str(1)?;
                Ok(Value::from(owned.replace(from, to)))
            }
            // Python accepts a single prefix or a tuple of prefixes.
            "startswith" => {
                let prefixes = str_or_seq(args.first().ok_or_else(unsupported)?)?;
                Ok(Value::from(
                    prefixes.iter().any(|prefix| owned.starts_with(prefix)),
                ))
            }
            "endswith" => {
                let suffixes = str_or_seq(args.first().ok_or_else(unsupported)?)?;
                Ok(Value::from(
                    suffixes.iter().any(|suffix| owned.ends_with(suffix)),
                ))
            }
            "find" => Ok(Value::from(
                owned.find(arg_str(0)?).map(|i| i as i64).unwrap_or(-1),
            )),
            // Paired with the `open` global above: the content is already
            // loaded, so `.read()` returns it unchanged.
            "read" => Ok(Value::from(owned)),
            "join" => {
                let seq = args.first().ok_or_else(unsupported)?;
                let mut parts: Vec<String> = Vec::new();
                if let Ok(iter) = seq.try_iter() {
                    for item in iter {
                        parts.push(py_str(&item));
                    }
                }
                Ok(Value::from(parts.join(&owned)))
            }
            _ => Err(unsupported()),
        };
    }

    // Maps first: `try_iter` on a map yields keys, so dict methods must be
    // matched before the sequence branch below.
    if let Some(pairs) = value
        .as_object()
        .filter(|object| matches!(object.repr(), ObjectRepr::Map))
        .and_then(|object| object.try_iter_pairs())
    {
        let entries: Vec<(minijinja::Value, minijinja::Value)> = pairs.collect();
        return match method {
            "get" => {
                let key = arg_str(0)?;
                for (k, v) in &entries {
                    if k.as_str() == Some(key) {
                        return Ok(v.clone());
                    }
                }
                Ok(args.get(1).cloned().unwrap_or(Value::UNDEFINED))
            }
            "keys" => Ok(Value::from(
                entries.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>(),
            )),
            "values" => Ok(Value::from(
                entries.iter().map(|(_, v)| v.clone()).collect::<Vec<_>>(),
            )),
            "items" => Ok(Value::from(
                entries
                    .iter()
                    .map(|(k, v)| Value::from(vec![k.clone(), v.clone()]))
                    .collect::<Vec<_>>(),
            )),
            _ => Err(unsupported()),
        };
    }

    if let Ok(iter) = value.try_iter() {
        let items: Vec<minijinja::Value> = iter.collect();
        return match method {
            // Jinja2 mutates in place and returns None. The template site
            // needing the side effect is pre-applied to the Rust context
            // (see `render_home_page`); here return Undefined so
            // `|default("", True)` renders "" exactly like Flask.
            "append" => Ok(Value::UNDEFINED),
            "index" => {
                let needle = args.first().ok_or_else(unsupported)?;
                let position = items.iter().position(|item| {
                    // Compare via debug rendering (covers str/num/bool).
                    format!("{item:?}") == format!("{needle:?}")
                });
                match position {
                    Some(i) => Ok(Value::from(i as i64)),
                    None => Err(Error::new(
                        ErrorKind::InvalidOperation,
                        "index(): not found",
                    )),
                }
            }
            "count" => {
                let needle = args.first().ok_or_else(unsupported)?;
                let count = items
                    .iter()
                    .filter(|item| format!("{item:?}") == format!("{needle:?}"))
                    .count();
                Ok(Value::from(count as i64))
            }
            _ => Err(unsupported()),
        };
    }

    Err(unsupported())
}

/// Python `int()` semantics for the template global.
fn py_int(value: &minijinja::Value) -> Option<i64> {
    if let Some(s) = value.as_str() {
        let trimmed = s.trim();
        if let Ok(int) = trimmed.parse::<i64>() {
            return Some(int);
        }
        return trimmed.parse::<f64>().ok().map(|f| f.trunc() as i64);
    }
    let rendered = format!("{value:?}");
    rendered
        .parse::<i64>()
        .ok()
        .or_else(|| rendered.parse::<f64>().ok().map(|f| f.trunc() as i64))
}

/// Python `str()` semantics for the template global (containers use `repr`).
fn py_str(value: &minijinja::Value) -> String {
    if let Some(s) = value.as_str() {
        return s.to_string();
    }
    py_repr(value)
}

/// Python `repr()` for numbers, bools, None, lists, and dicts.
///
/// Precedence matters: sequences and maps are truthy when non-empty, so
/// they must render BEFORE any truthiness test (else `['index']` becomes
/// `"True"` and the index page boots into the wrong folder — or none).
fn py_repr(value: &minijinja::Value) -> String {
    use minijinja::Value;
    if value.is_undefined() {
        return String::new();
    }
    if let Some(s) = value.as_str() {
        return format!("'{s}'");
    }
    if is_bool_or_none(value) {
        if is_none(value) {
            return "None".to_string();
        }
        return if format!("{value:?}").as_str() == "true" {
            "True".to_string()
        } else {
            "False".to_string()
        };
    }
    if is_number(value) {
        // Integers plain, floats shortest-roundtrip like Python repr.
        return format!("{value:?}").trim_matches('"').to_string();
    }
    if !is_map_value(value) {
        if let Ok(iter) = value.try_iter() {
            let items: Vec<String> = iter.map(|item| py_repr(&item)).collect();
            return format!("[{}]", items.join(", "));
        }
    }
    if let Some(pairs) = value
        .as_object()
        .filter(|object| matches!(object.repr(), ObjectRepr::Map))
        .and_then(|object| object.try_iter_pairs())
    {
        let items: Vec<String> = pairs
            .map(|(k, v)| format!("{}: {}", py_repr(&k), py_repr(&v)))
            .collect();
        return format!("{{{}}}", items.join(", "));
    }
    // Numbers and anything else: debug rendering matches closely enough
    // (integers plain, floats shortest-roundtrip like Python repr).
    let _ = Value::UNDEFINED;
    format!("{value:?}").trim_matches('"').to_string()
}

/// Accept a string or a sequence of strings (Python tuple form).
fn str_or_seq(value: &minijinja::Value) -> Result<Vec<String>, minijinja::Error> {
    if let Some(s) = value.as_str() {
        return Ok(vec![s.to_string()]);
    }
    let mut items = Vec::new();
    if let Ok(iter) = value.try_iter() {
        for item in iter {
            items.push(
                item.as_str()
                    .ok_or_else(|| {
                        minijinja::Error::new(
                            minijinja::ErrorKind::InvalidOperation,
                            "prefix must be a string",
                        )
                    })?
                    .to_string(),
            );
        }
        return Ok(items);
    }
    Err(minijinja::Error::new(
        minijinja::ErrorKind::InvalidOperation,
        "prefix must be a string or tuple of strings",
    ))
}

fn is_map_value(value: &minijinja::Value) -> bool {
    value
        .as_object()
        .map(|object| matches!(object.repr(), ObjectRepr::Map))
        .unwrap_or(false)
}

fn is_number(value: &minijinja::Value) -> bool {
    format!("{value:?}").parse::<f64>().is_ok()
}

fn is_bool_or_none(value: &minijinja::Value) -> bool {
    matches!(
        format!("{value:?}").as_str(),
        "true" | "false" | "none" | "null"
    )
}

fn is_none(value: &minijinja::Value) -> bool {
    matches!(format!("{value:?}").as_str(), "none" | "null")
}

/// Parse a Python literal (`['.exe']`, `[0, 100]`, `'text'`, numbers) as the
/// template `eval()` global. Only the shapes `args.jinja` produces are
/// supported — anything else is an error, never executed code.
fn eval_literal(source: &str) -> Option<minijinja::Value> {
    let parser = LiteralParser::new(source);
    parser.parse()
}

struct LiteralParser<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> LiteralParser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().peekable(),
        }
    }

    fn parse(mut self) -> Option<minijinja::Value> {
        let value = self.parse_value()?;
        self.skip_ws();
        if self.chars.next().is_some() {
            return None;
        }
        Some(value)
    }

    fn skip_ws(&mut self) {
        while self
            .chars
            .peek()
            .map(|c| c.is_whitespace())
            .unwrap_or(false)
        {
            self.chars.next();
        }
    }

    fn parse_value(&mut self) -> Option<minijinja::Value> {
        use minijinja::Value;
        self.skip_ws();
        match self.chars.peek()? {
            '[' => {
                self.chars.next();
                let mut items = Vec::new();
                loop {
                    self.skip_ws();
                    if self.chars.peek() == Some(&']') {
                        self.chars.next();
                        break;
                    }
                    items.push(self.parse_value()?);
                    self.skip_ws();
                    match self.chars.peek() {
                        Some(',') => {
                            self.chars.next();
                        }
                        Some(']') => continue,
                        _ => return None,
                    }
                }
                Some(Value::from(items))
            }
            '\'' | '"' => {
                let quote = self.chars.next()?;
                let mut text = String::new();
                while let Some(ch) = self.chars.next() {
                    if ch == quote {
                        return Some(Value::from(text));
                    }
                    if ch == '\\' {
                        if let Some(escaped) = self.chars.next() {
                            text.push(escaped);
                        }
                    } else {
                        text.push(ch);
                    }
                }
                None
            }
            _ => {
                let mut token = String::new();
                while let Some(&ch) = self.chars.peek() {
                    if ch == ',' || ch == ']' || ch.is_whitespace() {
                        break;
                    }
                    token.push(ch);
                    self.chars.next();
                }
                match token.as_str() {
                    "True" => Some(Value::from(true)),
                    "False" => Some(Value::from(false)),
                    "None" => Some(Value::from(())),
                    _ => token
                        .parse::<i64>()
                        .map(Value::from)
                        .or_else(|_| token.parse::<f64>().map(Value::from))
                        .ok(),
                }
            }
        }
    }
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

    match render_home_page(&config, &commands, &versions, &random_bg, is_exe) {
        Ok(html) => Html(html).into_response(),
        Err(message) => internal_error("An error occurred during a request", message, None),
    }
}

/// Build the `index.jinja` context and render it.
///
/// `config` is cloned before rendering: `index.jinja:533` appends
/// `"static/css/style.css"` to `config.front.themes` in Flask (a list
/// mutation minijinja values cannot express), so the entry is pre-pushed
/// here — same per-request result, since the config is re-read every time.
fn render_home_page(
    config: &Value,
    commands: &Value,
    versions: &Value,
    random_bg: &str,
    is_exe: bool,
) -> Result<String, String> {
    let mut config = config.clone();
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
    let template = env
        .get_template("index.jinja")
        .map_err(|e| format!("Cannot load index.jinja: {e}"))?;
    // A template bug must never kill the worker connection: contain any
    // engine panic as a 500 exactly like a render error.
    let rendered =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| template.render(&context)));
    match rendered {
        Ok(Ok(html)) => Ok(html),
        Ok(Err(e)) => Err(format!("Template render failed: {e:?}")),
        Err(_) => Err(
            "Template render panicked (minijinja/Jinja2 behavior gap — see docs/MIGRATION_RUST.md)"
                .to_string(),
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
        if config["settings"]["soundboard"]["enabled"]
            .as_bool()
            .unwrap_or(false)
        {
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
async fn save_buttons_only(
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
/// `easygui.diropenbox` → `rfd` folder picker (`""` on cancel, like Python).
async fn upload_folderpath() -> String {
    // Native dialogs are blocking; run off the async runtime.
    tokio::task::spawn_blocking(|| {
        rfd::FileDialog::new()
            .pick_folder()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default()
    })
    .await
    .unwrap_or_default()
}

/// Port of `upload_filepath` (`POST /upload_filepath`).
/// `easygui.fileopenbox` → `rfd` file picker (`""` on cancel, like Python).
async fn upload_filepath(Query(params): Query<HashMap<String, String>>) -> String {
    let filetypes = params.get("filetypes").cloned().unwrap_or_default();
    tokio::task::spawn_blocking(move || {
        let mut dialog = rfd::FileDialog::new();
        if !filetypes.is_empty() {
            // Python: `filetypes.split('_')` → `*ext` easygui patterns.
            let extensions: Vec<String> = filetypes
                .split('_')
                .map(|item| {
                    item.trim()
                        .trim_start_matches('.')
                        .trim_start_matches('*')
                        .to_string()
                })
                .filter(|item| !item.is_empty())
                .collect();
            let borrowed: Vec<&str> = extensions.iter().map(|s| s.as_str()).collect();
            if !borrowed.is_empty() {
                dialog = dialog.add_filter("files", &borrowed);
            }
        }
        dialog
            .pick_file()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default()
    })
    .await
    .unwrap_or_default()
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
        return Json(
            json!({"success": false, "message": text(Some("no_files_found_error"), None)}),
        )
        .into_response();
    };

    if info.as_deref() == Some("background_image") {
        save_rotated_copy(&format!(".config/user_uploads/{filename}"));
    }

    log().success(&format!("File '{filename}' uploaded successfully"));
    Json(json!({"success": true, "message": text(Some("downloaded_successfully"), None)}))
        .into_response()
}

/// Port of `create_folder` (`POST /create_folder`).
async fn create_folder(State(state): State<AppState>, Json(data): Json<Value>) -> Response {
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

/// SocketIO `/` namespace setup — port of the `@socketio.on(...)` handlers
/// in `app/server.py` (`connect`, `send`, `message_from_socket`).
fn socketio_layer(server_address: &str, server_port: u16) -> socketioxide::layer::SocketIoLayer {
    use socketioxide::extract::{Data, SocketRef};

    let (layer, io) = socketioxide::SocketIo::new_layer();
    let address = server_address.to_string();

    io.ns("/", move |socket: SocketRef| {
        let address = address.clone();
        async move {
            log().info(&format!("server connected at {address}:{server_port}"));

            socket.on(
                "send",
                |socket: SocketRef, Data::<Value>(data)| async move {
                    log().info(&format!("message received with : {data}"));
                    // Python `send(data, broadcast=True)` emits "message".
                    let _ = socket.broadcast().emit("message", &data).await;
                },
            );

            socket.on(
                "message_from_socket",
                |socket: SocketRef, Data::<Value>(data)| async move {
                    let message = data
                        .as_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| data.to_string());
                    log().info(&format!("Message from client: {message}"));
                    let owned = message.clone();
                    let result =
                        tokio::task::spawn_blocking(move || buttons::handle_command(&owned)).await;
                    if result.is_ok() {
                        // Python emits the ORIGINAL message, not the result.
                        let _ = socket.emit("json_data", &message);
                    }
                },
            );
        }
    });

    layer
}

/// Port of `send_data_route` (`POST /send-data`).
async fn send_data_route(Json(body): Json<Value>) -> Response {
    let message = body
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    // Command handling is blocking/sync (subprocesses, sleeps, sync HTTP for
    // Spotify/translate), like Python's gevent worker: run it off the runtime.
    let result = tokio::task::spawn_blocking(move || buttons::handle_command(&message)).await;
    match result {
        Ok(result) => {
            // NOTE: like Python's `send_data_route`, the HTTP path does not
            // emit SocketIO events; only `on_socket_message` emits `json_data`.
            Json(result).into_response()
        }
        Err(e) => internal_error(
            "An error occurred while handling a command",
            format!("Command task failed: {e}"),
            None,
        ),
    }
}

/// Port of `run_server`.
///
/// Python runs `on_start()` + firewall checks at import time; Rust runs them
/// at the top of this function (same order, same conditions).
pub async fn run_server() -> Result<(), ServerError> {
    let (config, _commands, local_ip) = on_start().await;
    set_global_variable("config", config.clone());

    // Python starts the mic loop at import time when enabled.
    soundboard::mic::start_if_enabled();

    // Python module level: firewall bypass + local IP log.
    if config["settings"]["automatic_firewall_bypass"].as_bool() == Some(true)
        && !check_firewall_permission()
    {
        fix_firewall_permission();
    }
    log().info(&format!("Local IP address detected: {local_ip}"));

    change_server_state(ServerState::Running);

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

    let host = get_args().host.clone().unwrap_or(local_ip);
    let port = get_port();
    let app = app.layer(socketio_layer(&host, port));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::utils::settings::get_config::test_support::{config_guard, seed_config};

    #[test]
    fn index_renders_with_realistic_context() {
        let _guard = config_guard();
        seed_config(&serde_json::json!({
            "url": {"port": 5000, "ip": "127.0.0.1"},
            "front": {"buttons": {}, "themes": [], "portrait_rotate": false},
            "settings": {"optimized_usage_display": false, "gpu_method": "None"},
        }));
        crate::app::utils::languages::init(
            "webdeck/translations",
            Some("webdeck/translations/misc"),
            "en_US",
        );
        let config: Value = serde_json::from_str(
            &std::fs::read_to_string("webdeck/config_default.json").expect("default config"),
        )
        .expect("parse default config");
        let commands: Value = serde_json::from_str(
            &std::fs::read_to_string("webdeck/commands.json").expect("commands"),
        )
        .expect("parse commands");
        let versions: Value = serde_json::from_str(
            &std::fs::read_to_string("webdeck/version.json").expect("versions"),
        )
        .expect("parse versions");
        let html = render_home_page(&config, &commands, &versions, "#141414", false)
            .expect("index.jinja renders");
        assert!(html.contains("WebDeck"), "missing brand marker");
        assert!(
            html.contains("static/css/style.css"),
            "missing base theme entry"
        );
    }

    #[test]
    fn folder_name_chain_matches_python() {
        // index.jinja boot: folder('{{str(config["front"]["buttons"].keys()).split("'")[1]}}')
        // must yield the first folder name (empty string = black page).
        let env = minijinja_env();
        let ctx = serde_json::json!({
            "config": {"front": {"buttons": {"index": [], "folder1": []}}},
        });
        let out = env
            .render_str(
                "{{ str(config['front']['buttons'].keys()).split(\"'\")[1] }}",
                &ctx,
            )
            .expect("renders");
        assert_eq!(out, "index");
    }

    #[test]
    fn eval_literal_parses_arg_shapes() {
        let exts = eval_literal("['.exe']").expect("list of str");
        assert_eq!(exts.try_iter().map(|i| i.count()).unwrap_or(0), 1);
        let nums = eval_literal("['0','100']").expect("list of str");
        assert_eq!(nums.try_iter().map(|i| i.count()).unwrap_or(0), 2);
        assert!(eval_literal("['a', 1, True]").is_some());
        assert!(eval_literal("not [ valid").is_none());
    }

    #[test]
    fn rotated_copy_swaps_dimensions() {
        let dir = std::env::temp_dir().join(format!("webdeck-rot-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let src = dir.join("bg.png");
        let img: image::RgbImage =
            image::ImageBuffer::from_fn(4, 2, |x, y| image::Rgb([x as u8, y as u8, 0]));
        img.save(&src).unwrap();
        save_rotated_copy(&src.to_string_lossy());
        let rotated = image::open(dir.join("bg-90.png")).unwrap();
        assert_eq!((rotated.width(), rotated.height()), (2, 4));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
