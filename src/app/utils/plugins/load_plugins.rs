//! Port of `app/utils/plugins/load_plugins.py`.
//!
//! Python dynamically imports `.config/plugins/*.py` and reads each
//! `WebDeckAddon.instance` (`_dict_doc` / `_dict_func` / `_addon_name`).
//! Dynamic Python loading has no Rust equivalent, so script plugins are
//! [`rhai`] scripts instead (`.config/plugins/*.rhai`, synced to
//! `temp/plugins` by the same copy step as Python).
//!
//! ## rhai plugin contract
//! Each script must define:
//! - `addon_name()` → plugin name (port of `_addon_name`)
//! - `addon_doc()` → map of `{command: doc}` (port of `_dict_doc`, merged
//!   into `commands[plugin_name]` like Python). Each value is either a doc
//!   string (legacy form) or a map (extended form):
//!   ```rhai
//!   #{
//!     greet: "says hi",
//!     add: #{
//!       description: "adds two numbers",
//!       command: "/add",
//!       args: [
//!         #{ TYPE: "input number['1','100']", label: "First" },
//!         #{ TYPE: "input text", label: "Note" },
//!       ],
//!     },
//!   }
//!   ```
//!   The map form declares the add-button form for the command: `args`
//!   entries use the same `TYPE` strings as `webdeck/commands.json`
//!   (`input text`, `input key`, `choice`, `input dropdown`, ...), plus an
//!   inline `label` shown verbatim (plugin labels have no `.lang` entries).
//!   `command` defaults to `/{name}` (the dispatch key) and `args` to `[]`.
//! - `addon_call(command, args)` → handles one command invocation with the
//!   `<|§|>`-split argument array (port of `_dict_func`, whose return values
//!   Python discards — rhai return values are likewise ignored)
//!
//! Host API available to scripts: `log_debug`, `log_info`, `log_notice`,
//! `log_warning`, `log_error`, `log_success`, `webdeck_command(cmd)`
//! (runs a button command, returns its response JSON; recursion past 8
//! nested plugin entries is refused), and `run_shell(cmd)` (runs a shell
//! command, returns its exit code). One persistent [`rhai::Scope`] per
//! plugin plays the role of Python module globals; reloading a plugin
//! recompiles it fresh (like `importlib.reload`).
//!
//! The same engine (built by [`script_engine`]) backs `/exec` script
//! execution in `buttons::exec`.
//!
//! `.py` plugins are still discovered but cannot be imported (logged, as
//! before); [`register_plugin_commands`] / [`plugin_commands`] form the
//! command registry that [`crate::app::buttons::commands::handle_command`]
//! dispatches through (replacing the `all_func` global).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use rhai::{Dynamic, Engine, Scope, AST};
use serde_json::Value;

use crate::app::utils::logger::log;

/// A plugin command handler: receives the `<|§|>`-split arguments.
pub type PluginFn = Arc<dyn Fn(&[String]) + Send + Sync>;

static REGISTRY: OnceLock<Mutex<HashMap<String, HashMap<String, PluginFn>>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<String, HashMap<String, PluginFn>>> {
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register (or replace) one plugin's `{command: handler}` map.
/// Port of the `all_func[plugin_name] = dict_func` assignment.
pub fn register_plugin_commands(plugin_name: &str, commands: HashMap<String, PluginFn>) {
    if let Ok(mut registry) = registry().lock() {
        registry.insert(plugin_name.to_string(), commands);
    }
}

/// Snapshot the plugin command registry (port of reading `all_func`).
pub fn plugin_commands() -> HashMap<String, HashMap<String, PluginFn>> {
    registry().lock().map(|r| r.clone()).unwrap_or_default()
}

/// Port of `load_plugins`.
///
/// Returns `(commands, loaded_plugin_names)`. `commands` gains entries only
/// for statically-registered plugins (dynamic `.py` import is deferred — see
/// module docs); the temp-dir refresh is kept 1:1.
pub fn load_plugins(mut commands: Value) -> (Value, Vec<String>) {
    let plugins_path = ".config/plugins";
    let temp_dir = "temp/plugins";

    if std::path::Path::new(temp_dir).exists() {
        let _ = std::fs::remove_dir_all(temp_dir);
    }
    if let Err(e) = std::fs::create_dir_all(temp_dir) {
        log().exception(
            &e,
            Some(&format!(
                "Error creating temp plugins directory '{temp_dir}'"
            )),
            false,
            true,
            false,
        );
    }

    // Same copy step as Python (keeps temp/plugins in sync for future loaders).
    if let Ok(entries) = walk_files(plugins_path) {
        for src in entries {
            if let Some(name) = src.file_name().and_then(|n| n.to_str()) {
                let dst = std::path::Path::new(temp_dir).join(name);
                if let Err(e) = std::fs::copy(&src, &dst) {
                    log().exception(
                        &e,
                        Some(&format!("Error copying plugin {}", src.display())),
                        true,
                        true,
                        false,
                    );
                }
            }
        }
    }

    let mut found: Vec<String> = Vec::new();
    if let Ok(entries) = walk_files(temp_dir) {
        for path in entries {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let module_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("?")
                .to_string();
            if ext == "py" {
                // Dynamic `.py` import has no Rust equivalent; discovered
                // but not imported (use `.rhai` scripts instead).
                log().info(&format!(
                    "Found plugin file (dynamic import deferred): {module_name}"
                ));
                found.push(module_name);
                continue;
            }
            if ext != "rhai" {
                continue;
            }
            match load_rhai_plugin(&path) {
                Ok((plugin_name, doc, funcs)) => {
                    log().info(&format!("Loaded plugin: {plugin_name}"));
                    register_plugin_commands(&plugin_name, funcs);
                    if let Some(map) = commands.as_object_mut() {
                        map.insert(plugin_name, doc);
                    }
                    found.push(module_name);
                }
                Err(e) => {
                    log().exception(
                        &e,
                        Some(&format!("Error importing module '{module_name}'")),
                        true,
                        true,
                        true,
                    );
                    continue;
                }
            }
        }
    }

    // Merge statically-registered plugins' docs into `commands`, mirroring
    // `commands[plugin_name] = dict_doc` (rhai plugins already inserted
    // theirs above, so only fill in the gaps).
    if let Some(map) = commands.as_object_mut() {
        for plugin_name in plugin_commands().keys() {
            if !map.contains_key(plugin_name) {
                map.insert(plugin_name.clone(), Value::Object(serde_json::Map::new()));
                log().info(&format!("Loaded plugin: {plugin_name}"));
            }
        }
    }

    (commands, found)
}

/// One loaded rhai plugin: engine + compiled script + persistent scope
/// (the scope is the script's module-global state across calls).
struct RhaiPlugin {
    engine: Engine,
    ast: AST,
    scope: Scope<'static>,
}

// Thread-local plugin-entry depth (guards `webdeck_command` recursion).
thread_local! {
    static PLUGIN_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Build a script engine with the WebDeck host API registered.
/// Shared by `.rhai` plugins and `/exec` script execution.
pub(crate) fn script_engine() -> Engine {
    let mut engine = Engine::new();
    engine.register_fn("log_debug", |msg: &str| log().debug(msg));
    engine.register_fn("log_info", |msg: &str| log().info(msg));
    engine.register_fn("log_notice", |msg: &str| log().notice(msg));
    engine.register_fn("log_warning", |msg: &str| log().warning(msg));
    engine.register_fn("log_error", |msg: &str| log().error(msg));
    engine.register_fn("log_success", |msg: &str| log().success(msg));
    engine.register_fn("webdeck_command", |cmd: &str| -> String {
        let depth = PLUGIN_DEPTH.with(|d| {
            let depth = d.get() + 1;
            d.set(depth);
            depth
        });
        if depth > 8 {
            PLUGIN_DEPTH.with(|d| d.set(d.get() - 1));
            log().error("webdeck_command: plugin recursion limit reached");
            return r#"{"success": false, "message": "plugin recursion limit reached"}"#
                .to_string();
        }
        let response = serde_json::to_string(&crate::app::buttons::commands::handle_command(cmd))
            .unwrap_or_else(|_| r#"{"success": false}"#.to_string());
        PLUGIN_DEPTH.with(|d| d.set(d.get() - 1));
        response
    });
    engine.register_fn("run_shell", |cmd: &str| -> i64 {
        #[cfg(windows)]
        let result = std::process::Command::new("cmd").args(["/C", cmd]).status();
        #[cfg(not(windows))]
        let result = std::process::Command::new("sh").args(["-c", cmd]).status();
        match result {
            Ok(status) => status.code().unwrap_or(-1) as i64,
            Err(_) => -1,
        }
    });
    engine
}

/// Convert a rhai value to JSON (port of the `dict_doc` `_to_dict` mapping).
fn dynamic_to_json(value: Dynamic) -> Value {
    if value.is_string() {
        Value::from(value.into_string().unwrap_or_default())
    } else if value.is_int() {
        Value::from(value.as_int().unwrap_or(0))
    } else if value.is_float() {
        serde_json::Number::from_f64(value.as_float().unwrap_or(0.0))
            .map(Value::from)
            .unwrap_or(Value::Null)
    } else if value.is_bool() {
        Value::from(value.as_bool().unwrap_or(false))
    } else if value.is_array() {
        Value::from(
            value
                .into_array()
                .unwrap_or_default()
                .into_iter()
                .map(dynamic_to_json)
                .collect::<Vec<_>>(),
        )
    } else if value.is_map() {
        Value::from(
            value
                .cast::<rhai::Map>()
                .into_iter()
                .map(|(k, v)| (k.to_string(), dynamic_to_json(v)))
                .collect::<serde_json::Map<String, Value>>(),
        )
    } else if value.is_unit() {
        Value::Null
    } else {
        Value::from(value.to_string())
    }
}

/// Normalize one `addon_doc()` entry into a frontend command object.
///
/// - string → `{command: "/{key}", args: [], description}` (legacy form)
/// - map → passed through with `command` defaulting to `/{key}` and `args`
///   defaulting to `[]` (a non-array `args` is replaced with `[]`)
/// - anything else → `{command: "/{key}", args: []}` plus a warning
fn normalize_plugin_doc_entry(key: &str, value: Dynamic) -> Value {
    let default_command = Value::from(format!("/{key}"));
    if value.is_string() {
        return serde_json::json!({
            "command": default_command,
            "args": [],
            "description": value.into_string().unwrap_or_default(),
        });
    }
    if value.is_map() {
        let mut obj = match dynamic_to_json(value) {
            Value::Object(map) => map,
            _ => serde_json::Map::new(),
        };
        let needs_command = obj
            .get("command")
            .and_then(Value::as_str)
            .is_none_or(|s| s.is_empty());
        if needs_command {
            obj.insert("command".to_string(), default_command);
        }
        if !obj.get("args").is_some_and(Value::is_array) {
            obj.insert("args".to_string(), Value::Array(vec![]));
        }
        return Value::Object(obj);
    }
    log().warning(&format!(
        "addon_doc entry '{key}': expected a string or map, ignoring value"
    ));
    serde_json::json!({"command": default_command, "args": []})
}

/// Compile one `.rhai` plugin: returns (addon name, doc JSON, commands).
fn load_rhai_plugin(
    path: &std::path::Path,
) -> Result<(String, Value, HashMap<String, PluginFn>), String> {
    let source = std::fs::read_to_string(path).map_err(|e| format!("cannot read plugin: {e}"))?;
    let engine = script_engine();
    let ast = engine
        .compile(source)
        .map_err(|e| format!("cannot compile plugin: {e}"))?;
    let mut scope = Scope::new();

    let name: String = engine
        .call_fn(&mut scope, &ast, "addon_name", ())
        .map_err(|e| format!("addon_name() failed: {e}"))?;
    let doc: rhai::Map = engine
        .call_fn(&mut scope, &ast, "addon_doc", ())
        .map_err(|e| format!("addon_doc() failed: {e}"))?;

    let vm = Arc::new(Mutex::new(RhaiPlugin { engine, ast, scope }));
    let mut funcs: HashMap<String, PluginFn> = HashMap::new();
    for command in doc.keys() {
        let vm = Arc::clone(&vm);
        let command = command.to_string();
        let plugin = name.clone();
        let key = command.clone();
        let handler: PluginFn = Arc::new(move |args: &[String]| {
            let args: Vec<Dynamic> = args.iter().map(|s| Dynamic::from(s.clone())).collect();
            let mut vm = match vm.lock() {
                Ok(vm) => vm,
                Err(_) => return,
            };
            let RhaiPlugin { engine, ast, scope } = &mut *vm;
            // Return values are discarded, like Python's `func(*args)`.
            if let Err(e) =
                engine.call_fn::<Dynamic>(scope, ast, "addon_call", (command.clone(), args))
            {
                log().exception(
                    &e,
                    Some(&format!("Error in plugin '{plugin}' command '{command}'")),
                    true,
                    true,
                    true,
                );
            }
        });
        funcs.insert(key, handler);
    }

    let doc_json = Value::from(
        doc.into_iter()
            .map(|(k, v)| {
                let key = k.to_string();
                let entry = normalize_plugin_doc_entry(&key, v);
                (key, entry)
            })
            .collect::<serde_json::Map<String, Value>>(),
    );
    Ok((name, doc_json, funcs))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_plugin(source: &str) -> std::path::PathBuf {
        // Unique file per test: parallel tests must not share one path.
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join("webdeck_rhai_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(format!("demo_{}_{id}.rhai", std::process::id()));
        std::fs::write(&path, source).unwrap();
        path
    }

    #[test]
    fn rhai_plugin_contract_end_to_end() {
        let path = write_plugin(
            r#"
            fn addon_name() { "demo" }
            fn addon_doc() { #{ greet: "says hi", add: "adds" } }
            fn addon_call(cmd, args) {
                log_info("called " + cmd);
                if cmd == "add" { return args.len(); }
            }
        "#,
        );
        let (name, doc, funcs) = load_rhai_plugin(&path).expect("plugin loads");
        assert_eq!(name, "demo");
        // Legacy string docs normalize to arg-less command objects.
        assert_eq!(doc["greet"]["command"], Value::from("/greet"));
        assert_eq!(doc["greet"]["args"], Value::Array(vec![]));
        assert_eq!(doc["greet"]["description"], Value::from("says hi"));
        assert!(funcs.contains_key("greet") && funcs.contains_key("add"));
        // Dispatch works and discards the return value.
        funcs["add"](&["1".to_string(), "2".to_string()]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn plugin_doc_map_form_declares_args() {
        let path = write_plugin(
            r#"
            fn addon_name() { "calc" }
            fn addon_doc() { #{
                add: #{
                    description: "adds two numbers",
                    args: [
                        #{ TYPE: "input number['1','100']", label: "First" },
                        #{ TYPE: "input text", label: "Note", placeholder: "hi" }
                    ]
                },
                ping: #{ description: "no args here" },
                broken: #{ args: "not-an-array" }
            } }
            fn addon_call(cmd, args) {}
        "#,
        );
        let (_, doc, funcs) = load_rhai_plugin(&path).expect("plugin loads");
        assert_eq!(doc["add"]["command"], Value::from("/add"));
        assert_eq!(
            doc["add"]["args"][0]["TYPE"],
            Value::from("input number['1','100']")
        );
        assert_eq!(doc["add"]["args"][0]["label"], Value::from("First"));
        assert_eq!(doc["add"]["args"][1]["placeholder"], Value::from("hi"));
        assert_eq!(doc["add"]["description"], Value::from("adds two numbers"));
        assert_eq!(doc["ping"]["command"], Value::from("/ping"));
        assert_eq!(doc["ping"]["args"], Value::Array(vec![]));
        assert_eq!(doc["broken"]["args"], Value::Array(vec![]));
        assert!(funcs.contains_key("add"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn plugin_doc_wrong_type_falls_back_to_empty_command() {
        let path = write_plugin(
            r#"
            fn addon_name() { "weird" }
            fn addon_doc() { #{ num: 42 } }
            fn addon_call(cmd, args) {}
        "#,
        );
        let (_, doc, funcs) = load_rhai_plugin(&path).expect("plugin loads");
        assert_eq!(doc["num"]["command"], Value::from("/num"));
        assert_eq!(doc["num"]["args"], Value::Array(vec![]));
        assert!(funcs.contains_key("num"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn broken_plugin_reports_import_error() {
        let path = write_plugin("fn addon_name() { 1 + }");
        assert!(load_rhai_plugin(&path).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn dynamic_to_json_converts_nesting() {
        let mut map = rhai::Map::new();
        map.insert("n".into(), Dynamic::from(3_i64));
        map.insert("s".into(), Dynamic::from("x"));
        let value = dynamic_to_json(Dynamic::from(map));
        assert_eq!(value, serde_json::json!({"n": 3, "s": "x"}));
    }
}

fn walk_files(root: &str) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![std::path::PathBuf::from(root)];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                files.push(path);
            }
        }
    }
    Ok(files)
}
