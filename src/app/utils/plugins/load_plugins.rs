//! Port of `app/utils/plugins/load_plugins.py`.
//!
//! Python dynamically imports `.config/plugins/*.py` and reads each
//! `WebDeckAddon.instance` (`_dict_doc` / `_dict_func` / `_addon_name`).
//! Dynamic Python loading has no direct Rust equivalent, so the port splits
//! the job:
//! - [`load_plugins`] keeps the discovery side: it refreshes `temp/plugins`
//!   from `.config/plugins` (same copy step) and reports found plugin files.
//! - [`register_plugin_commands`] / [`plugin_commands`] form the static
//!   command registry that [`crate::app::buttons::commands::handle_command`]
//!   dispatches through (replacing the `all_func` global).
//!
//! Planned follow-up: load compiled plugins via `libloading` (cdylib
//! `WebDeckAddon` ABI) and/or script plugins via `rhai`, both registering
//! through [`register_plugin_commands`].

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

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
            Some(&format!("Error creating temp plugins directory '{temp_dir}'")),
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
                    log().exception(&e, Some(&format!("Error copying plugin {}", src.display())), true, true, false);
                }
            }
        }
    }

    let mut found: Vec<String> = Vec::new();
    if let Ok(entries) = walk_files(temp_dir) {
        for path in entries {
            let is_py = path.extension().and_then(|e| e.to_str()) == Some("py");
            if !is_py {
                continue;
            }
            let module_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("?")
                .to_string();
            // TODO(port): dynamic plugin loading (libloading/rhai). Until
            // then, .py plugins are discovered but not imported.
            log().info(&format!(
                "Found plugin file (dynamic import deferred): {module_name}"
            ));
            found.push(module_name);
        }
    }

    // Merge statically-registered plugins' docs into `commands`, mirroring
    // `commands[plugin_name] = dict_doc`.
    if let Some(map) = commands.as_object_mut() {
        for plugin_name in plugin_commands().keys() {
            map.entry(plugin_name.clone())
                .or_insert(Value::Object(serde_json::Map::new()));
            log().info(&format!("Loaded plugin: {plugin_name}"));
        }
    }

    (commands, found)
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
