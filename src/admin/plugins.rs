use super::client::WebDeckAdminClient;
use super::error::{AdminError, ErrorKind, Result};
use super::output::Outcome;
use crate::contracts::{PluginManifest, RuntimeSnapshot};
use crate::runtime::plugins::{load_package, RuntimePlugin};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const MAX_FILES: usize = 256;
const MAX_BYTES: u64 = 64 * 1024 * 1024;
const MAX_DEPTH: usize = 16;

const INDEX_TEMPLATE: &str = r#"export function invoke_action(action, args, ctx) {
  if (action === "echo") {
    return {echo: String(args.text ?? "")};
  }
  throw new Error("unknown action: " + action);
}
"#;

pub struct PluginService {
    client: WebDeckAdminClient,
    config_dir: PathBuf,
    root: PathBuf,
}

impl WebDeckAdminClient {
    pub fn plugins(&self, config_dir: impl Into<PathBuf>) -> PluginService {
        let config_dir = config_dir.into();
        let root = config_dir.join("plugins");
        PluginService {
            client: self.clone(),
            config_dir,
            root,
        }
    }
}

fn plugin_error(message: impl Into<String>, details: Value) -> AdminError {
    AdminError::with_details(ErrorKind::Plugin, message, details)
}

fn pluginize(error: AdminError) -> AdminError {
    match error.kind() {
        ErrorKind::Connection | ErrorKind::Authentication => error,
        _ => error.into_kind(ErrorKind::Plugin),
    }
}

fn is_builtin(id: &str) -> bool {
    id.starts_with("builtin.")
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        && !matches!(id, "." | "..")
        && !id.starts_with("builtin.")
}

fn manifest_value(manifest: &PluginManifest) -> Value {
    serde_json::to_value(manifest).unwrap_or(Value::Null)
}

fn clean(path: &Path) {
    let _ = std::fs::remove_dir_all(path);
}

fn copy_tree(
    src: &Path,
    dst: &Path,
    files: &mut usize,
    bytes: &mut u64,
    depth: usize,
) -> Result<()> {
    if depth > MAX_DEPTH {
        return Err(plugin_error(
            "Plugin package nesting is too deep",
            json!({"path": src.display().to_string()}),
        ));
    }
    let meta = std::fs::symlink_metadata(src).map_err(|e| {
        plugin_error(
            format!("Cannot read {}", src.display()),
            json!({"reason": e.to_string()}),
        )
    })?;
    if meta.file_type().is_symlink() {
        return Err(plugin_error(
            "Plugin packages cannot contain symbolic links",
            json!({"path": src.display().to_string()}),
        ));
    }
    if meta.is_dir() {
        std::fs::create_dir_all(dst).map_err(|e| {
            plugin_error(
                format!("Cannot create {}", dst.display()),
                json!({"reason": e.to_string()}),
            )
        })?;
        let entries = std::fs::read_dir(src).map_err(|e| {
            plugin_error(
                format!("Cannot read {}", src.display()),
                json!({"reason": e.to_string()}),
            )
        })?;
        for entry in entries {
            let entry = entry.map_err(|e| {
                plugin_error(
                    "Cannot read package entry",
                    json!({"reason": e.to_string()}),
                )
            })?;
            copy_tree(
                &entry.path(),
                &dst.join(entry.file_name()),
                files,
                bytes,
                depth + 1,
            )?;
        }
        Ok(())
    } else if meta.is_file() {
        *files += 1;
        *bytes += meta.len();
        if *files > MAX_FILES {
            return Err(plugin_error(
                format!("Plugin packages are limited to {MAX_FILES} files"),
                json!({"path": src.display().to_string()}),
            ));
        }
        if *bytes > MAX_BYTES {
            return Err(plugin_error(
                format!("Plugin packages are limited to {MAX_BYTES} bytes"),
                json!({"path": src.display().to_string()}),
            ));
        }
        std::fs::copy(src, dst).map_err(|e| {
            plugin_error(
                format!("Cannot copy {}", src.display()),
                json!({"reason": e.to_string()}),
            )
        })?;
        Ok(())
    } else {
        Err(plugin_error(
            "Plugin packages may only contain regular files and directories",
            json!({"path": src.display().to_string()}),
        ))
    }
}

fn load_checked(directory: &Path) -> Result<RuntimePlugin> {
    load_package(directory).map_err(|e| {
        plugin_error(
            format!("Plugin package is invalid: {}", e.message),
            json!({"path": directory.display().to_string(), "reason": e.message}),
        )
    })
}

fn state_data(id: &str, snapshot: &RuntimeSnapshot) -> Value {
    json!({
        "id": id,
        "enabled": !snapshot.disabled_plugins.iter().any(|d| d == id),
        "loaded": snapshot.loaded_plugins.iter().any(|l| l == id),
        "healthy": snapshot.healthy
    })
}

impl PluginService {
    fn stage_root(&self) -> PathBuf {
        self.config_dir.join(".webdeck-staging")
    }
    fn rollback_root(&self) -> PathBuf {
        self.config_dir.join(".webdeck-rollback")
    }
    fn prepare_stage(&self, source: &Path, id: &str) -> Result<PathBuf> {
        let stage = self.stage_root().join(id);
        clean(&stage);
        if let Some(parent) = stage.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                plugin_error(
                    format!("Cannot create {}", parent.display()),
                    json!({"reason": e.to_string()}),
                )
            })?;
        }
        let mut files = 0;
        let mut bytes = 0;
        copy_tree(source, &stage, &mut files, &mut bytes, 0)?;
        load_checked(&stage)?;
        Ok(stage)
    }
    fn clear_stage(&self, id: &str) {
        clean(&self.stage_root().join(id));
        let _ = std::fs::remove_dir(self.stage_root());
    }
    fn clear_rollback(&self, id: &str) {
        clean(&self.rollback_root().join(id));
        let _ = std::fs::remove_dir(self.rollback_root());
    }
    fn restore(&self, id: &str) -> bool {
        let rollback = self.rollback_root().join(id);
        let target = self.root.join(id);
        if !rollback.exists() {
            return false;
        }
        clean(&target);
        std::fs::rename(&rollback, &target).is_ok()
    }
    async fn recover_runtime(&self) -> bool {
        self.client.runtime_reload().await.is_ok()
    }
    pub async fn list(&self) -> Result<Outcome> {
        let snapshot = self.client.runtime_status().await.map_err(pluginize)?;
        let plugins: Vec<Value> = snapshot
            .plugins
            .iter()
            .map(|p| {
                json!({
                    "id": p.id,
                    "version": p.version,
                    "backend": p.backend,
                    "origin": p.origin,
                    "builtin": is_builtin(&p.id),
                    "enabled": !snapshot.disabled_plugins.contains(&p.id),
                    "loaded": snapshot.loaded_plugins.contains(&p.id),
                    "actions": p.actions.iter().map(|a| a.id.clone()).collect::<Vec<_>>()
                })
            })
            .collect();
        let count = plugins.len();
        let disabled = snapshot.disabled_plugins.len();
        Ok(Outcome::ok(
            json!({
                "plugins": plugins,
                "healthy": snapshot.healthy,
                "disabled": snapshot.disabled_plugins
            }),
            format!(
                "{count} plugin(s), {disabled} disabled, healthy: {}.",
                snapshot.healthy
            ),
        ))
    }
    pub async fn inspect(&self, id: &str) -> Result<Outcome> {
        let snapshot = self.client.runtime_status().await.map_err(pluginize)?;
        let path = self.root.join(id);
        if let Some(manifest) = snapshot.plugins.iter().find(|p| p.id == id) {
            let data = json!({
                "id": manifest.id,
                "version": manifest.version,
                "backend": manifest.backend,
                "origin": manifest.origin,
                "builtin": is_builtin(id),
                "enabled": !snapshot.disabled_plugins.iter().any(|d| d == id),
                "loaded": snapshot.loaded_plugins.iter().any(|l| l == id),
                "healthy": snapshot.healthy,
                "path": path.display().to_string(),
                "manifest": manifest_value(manifest)
            });
            return Ok(Outcome::ok(
                data,
                format!(
                    "Plugin '{id}' {} ({}) installed at {}.",
                    manifest.version,
                    manifest.backend,
                    path.display()
                ),
            ));
        }
        if !is_builtin(id) && path.join("webdeck.json").exists() {
            let package = load_checked(&path)?;
            let data = json!({
                "id": package.manifest.id,
                "version": package.manifest.version,
                "backend": package.manifest.backend,
                "origin": package.manifest.origin,
                "builtin": false,
                "enabled": Value::Null,
                "loaded": false,
                "healthy": snapshot.healthy,
                "path": path.display().to_string(),
                "manifest": manifest_value(&package.manifest)
            });
            return Ok(Outcome::ok(
                data,
                format!(
                    "Plugin '{id}' {} is on disk but not loaded by the runtime.",
                    package.manifest.version
                ),
            ));
        }
        Err(AdminError::with_details(
            ErrorKind::NotFound,
            format!("Plugin '{id}' is not installed"),
            json!({"id": id}),
        ))
    }
    pub fn validate_package(&self, path_or_id: &str) -> Result<Outcome> {
        let directory = if Path::new(path_or_id).join("webdeck.json").exists() {
            PathBuf::from(path_or_id)
        } else {
            self.root.join(path_or_id)
        };
        if !directory.exists() {
            return Err(AdminError::with_details(
                ErrorKind::NotFound,
                format!("Plugin package not found: {}", directory.display()),
                json!({"path": directory.display().to_string()}),
            ));
        }
        let package = load_checked(&directory)?;
        let data = json!({
            "valid": true,
            "id": package.manifest.id,
            "version": package.manifest.version,
            "backend": package.manifest.backend,
            "digest": package.manifest.digest,
            "path": directory.display().to_string()
        });
        Ok(Outcome::ok(
            data,
            format!(
                "Plugin package '{}' is valid ({} {}).",
                package.manifest.id, package.manifest.version, package.manifest.backend
            ),
        ))
    }
    pub async fn set_enabled(&self, id: &str, enabled: bool) -> Result<Outcome> {
        if is_builtin(id) {
            if enabled {
                return Ok(Outcome::ok(
                    json!({"id": id, "enabled": true, "applied": false, "builtin": true}),
                    format!("Builtin plugin '{id}' is always enabled."),
                ));
            }
            return Err(plugin_error(
                format!("Builtin plugin '{id}' cannot be disabled"),
                json!({"id": id}),
            ));
        }
        let snapshot = self.client.runtime_status().await.map_err(pluginize)?;
        if !snapshot.plugins.iter().any(|p| p.id == id) {
            return Err(AdminError::with_details(
                ErrorKind::NotFound,
                format!("Plugin '{id}' is not installed"),
                json!({"id": id}),
            ));
        }
        let current = !snapshot.disabled_plugins.iter().any(|d| d == id);
        if current == enabled {
            let state = if enabled { "enabled" } else { "disabled" };
            return Ok(Outcome::ok(
                json!({"id": id, "enabled": enabled, "applied": false, "healthy": snapshot.healthy}),
                format!("Plugin '{id}' is already {state}."),
            ));
        }
        let snapshot = self
            .client
            .set_plugin_enabled(id, enabled)
            .await
            .map_err(pluginize)?;
        let state = if enabled { "enabled" } else { "disabled" };
        let mut data = state_data(id, &snapshot);
        data["applied"] = json!(true);
        Ok(Outcome::ok(data, format!("Plugin '{id}' {state}.")))
    }
    pub async fn reload(&self) -> Result<Outcome> {
        let snapshot = self.client.runtime_reload().await.map_err(pluginize)?;
        let data = json!({
            "healthy": snapshot.healthy,
            "loaded_plugins": snapshot.loaded_plugins,
            "plugins": snapshot.plugins.len()
        });
        Ok(Outcome::ok(
            data,
            format!(
                "Runtime reloaded: {} plugin(s), healthy: {}.",
                snapshot.plugins.len(),
                snapshot.healthy
            ),
        ))
    }
    pub fn init(&self, id: &str, dir: Option<PathBuf>, version: &str) -> Result<Outcome> {
        if !valid_id(id) {
            return Err(AdminError::invalid_arguments(format!(
                "Invalid plugin id '{id}'"
            )));
        }
        semver::Version::parse(version).map_err(|_| {
            AdminError::invalid_arguments(format!("Invalid plugin version '{version}'"))
        })?;
        let target = dir.unwrap_or_else(|| PathBuf::from(id));
        if target.exists() {
            return Err(AdminError::with_details(
                ErrorKind::InvalidArguments,
                format!("{} already exists", target.display()),
                json!({"path": target.display().to_string()}),
            ));
        }
        if target.file_name().and_then(|n| n.to_str()) != Some(id) {
            return Err(AdminError::invalid_arguments(format!(
                "Plugin directory must be named '{id}'"
            )));
        }
        std::fs::create_dir_all(&target).map_err(|e| {
            AdminError::invalid_arguments(format!("Cannot create {}: {e}", target.display()))
        })?;
        let index = INDEX_TEMPLATE.as_bytes();
        let digest = format!("{:x}", Sha256::digest(index));
        std::fs::write(target.join("index.js"), index).map_err(|e| {
            AdminError::invalid_arguments(format!(
                "Cannot write {}: {e}",
                target.join("index.js").display()
            ))
        })?;
        let manifest = json!({
            "schema_version": 2,
            "id": id,
            "version": version,
            "entry": "index.js",
            "backend": "sandbox_js",
            "digest": digest,
            "origin": "local",
            "contract": "",
            "actions": [
                {
                    "id": "echo",
                    "label": "Echo text",
                    "capabilities": ["read"],
                    "arguments": {"text": {"type": "string", "required": true}},
                    "result": {"type": "object", "required": true}
                }
            ]
        });
        let bytes = serde_json::to_vec_pretty(&manifest)
            .map_err(|_| plugin_error("Cannot encode manifest", json!({})))?;
        std::fs::write(target.join("webdeck.json"), &bytes).map_err(|e| {
            AdminError::invalid_arguments(format!(
                "Cannot write {}: {e}",
                target.join("webdeck.json").display()
            ))
        })?;
        let package = load_package(&target).map_err(|e| {
            clean(&target);
            plugin_error(
                format!("Generated plugin package is invalid: {}", e.message),
                json!({"path": target.display().to_string(), "reason": e.message}),
            )
        })?;
        let data = json!({
            "id": package.manifest.id,
            "version": package.manifest.version,
            "entry": package.manifest.entry,
            "digest": package.manifest.digest,
            "path": target.display().to_string(),
            "scaffolded": true
        });
        Ok(Outcome::ok(
            data,
            format!("Created plugin '{id}' {version} at {}.", target.display()),
        ))
    }
    pub async fn install(&self, path: &str) -> Result<Outcome> {
        let source = PathBuf::from(path);
        let package = load_checked(&source)?;
        let id = package.manifest.id.clone();
        let version = package.manifest.version.clone();
        let target = self.root.join(&id);
        if target.exists() {
            return Err(plugin_error(
                format!("Plugin '{id}' is already installed; use plugin update"),
                json!({"id": id}),
            ));
        }
        let stage = self.prepare_stage(&source, &id)?;
        std::fs::create_dir_all(&self.root).map_err(|e| {
            plugin_error(
                format!("Cannot create {}", self.root.display()),
                json!({"reason": e.to_string()}),
            )
        })?;
        std::fs::rename(&stage, &target).map_err(|e| {
            self.clear_stage(&id);
            plugin_error(
                format!("Cannot install plugin '{id}'"),
                json!({"reason": e.to_string()}),
            )
        })?;
        self.clear_stage(&id);
        match self.client.runtime_reload().await {
            Ok(snapshot) => {
                let data = json!({
                    "operation": "plugin_install",
                    "id": id,
                    "version": version,
                    "path": target.display().to_string(),
                    "installed": true,
                    "healthy": snapshot.healthy
                });
                Ok(Outcome::ok(
                    data,
                    format!("Installed plugin '{id}' {version} and reloaded the runtime."),
                ))
            }
            Err(error) => {
                clean(&target);
                self.clear_stage(&id);
                let recovered = self.recover_runtime().await;
                Err(pluginize(error).attach_details(json!({
                    "installed": false,
                    "id": id,
                    "runtime_recovered": recovered
                })))
            }
        }
    }
    pub async fn update(&self, path: &str) -> Result<Outcome> {
        let source = PathBuf::from(path);
        let package = load_checked(&source)?;
        let id = package.manifest.id.clone();
        let version = package.manifest.version.clone();
        let target = self.root.join(&id);
        if !target.exists() {
            return Err(AdminError::with_details(
                ErrorKind::NotFound,
                format!("Plugin '{id}' is not installed"),
                json!({"id": id}),
            ));
        }
        let stage = self.prepare_stage(&source, &id)?;
        let rollback = self.rollback_root().join(&id);
        clean(&rollback);
        if let Some(parent) = rollback.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                self.clear_stage(&id);
                plugin_error(
                    format!("Cannot create {}", parent.display()),
                    json!({"reason": e.to_string()}),
                )
            })?;
        }
        std::fs::rename(&target, &rollback).map_err(|e| {
            self.clear_stage(&id);
            plugin_error(
                format!("Cannot stage rollback for '{id}'"),
                json!({"reason": e.to_string()}),
            )
        })?;
        if let Err(e) = std::fs::rename(&stage, &target) {
            let restored = std::fs::rename(&rollback, &target).is_ok();
            self.clear_stage(&id);
            self.clear_rollback(&id);
            return Err(plugin_error(
                format!("Cannot update plugin '{id}'"),
                json!({"reason": e.to_string(), "restored": restored}),
            ));
        }
        self.clear_stage(&id);
        match self.client.runtime_reload().await {
            Ok(snapshot) => {
                self.clear_rollback(&id);
                let data = json!({
                    "operation": "plugin_update",
                    "id": id,
                    "version": version,
                    "path": target.display().to_string(),
                    "updated": true,
                    "healthy": snapshot.healthy
                });
                Ok(Outcome::ok(
                    data,
                    format!("Updated plugin '{id}' to {version} and reloaded the runtime."),
                ))
            }
            Err(error) => {
                let restored = self.restore(&id);
                self.clear_stage(&id);
                self.clear_rollback(&id);
                let recovered = self.recover_runtime().await;
                Err(pluginize(error).attach_details(json!({
                    "updated": false,
                    "id": id,
                    "restored": restored,
                    "runtime_recovered": recovered
                })))
            }
        }
    }
    pub async fn uninstall(&self, id: &str) -> Result<Outcome> {
        if is_builtin(id) {
            return Err(plugin_error(
                format!("Builtin plugin '{id}' cannot be uninstalled"),
                json!({"id": id}),
            ));
        }
        let target = self.root.join(id);
        if !target.exists() {
            return Err(AdminError::with_details(
                ErrorKind::NotFound,
                format!("Plugin '{id}' is not installed"),
                json!({"id": id}),
            ));
        }
        let rollback = self.rollback_root().join(id);
        clean(&rollback);
        if let Some(parent) = rollback.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                plugin_error(
                    format!("Cannot create {}", parent.display()),
                    json!({"reason": e.to_string()}),
                )
            })?;
        }
        std::fs::rename(&target, &rollback).map_err(|e| {
            plugin_error(
                format!("Cannot stage removal of '{id}'"),
                json!({"reason": e.to_string()}),
            )
        })?;
        match self.client.runtime_reload().await {
            Ok(snapshot) => {
                self.clear_rollback(id);
                let data = json!({
                    "operation": "plugin_uninstall",
                    "id": id,
                    "removed": true,
                    "healthy": snapshot.healthy
                });
                Ok(Outcome::ok(
                    data,
                    format!("Uninstalled plugin '{id}' and reloaded the runtime."),
                ))
            }
            Err(error) => {
                let restored = self.restore(id);
                self.clear_rollback(id);
                let recovered = self.recover_runtime().await;
                Err(pluginize(error).attach_details(json!({
                    "removed": false,
                    "id": id,
                    "restored": restored,
                    "runtime_recovered": recovered
                })))
            }
        }
    }
}
