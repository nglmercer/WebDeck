use crate::{
    contracts::{PluginArgument, PluginManifest},
    domain::{self, Error, Result},
    storage::Assets,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct RuntimePlugin {
    pub manifest: PluginManifest,
    pub root: PathBuf,
    pub source: Option<String>,
}
fn read_bounded(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|_| Error::invalid())?
        .take(65537)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::invalid())?;
    if bytes.len() > 65536 {
        return Err(Error::invalid());
    }
    Ok(bytes)
}
pub fn confined(root: &Path, relative: &str) -> Result<PathBuf> {
    if relative.is_empty()
        || relative == "."
        || relative == ".."
        || Path::new(relative).is_absolute()
        || Path::new(relative)
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(Error::invalid());
    }
    let root = fs::canonicalize(root).map_err(|_| Error::invalid())?;
    let path = root.join(relative);
    if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(Error::invalid());
    }
    let path = fs::canonicalize(path).map_err(|_| Error::invalid())?;
    if !path.starts_with(&root) || !path.is_file() {
        return Err(Error::invalid());
    }
    Ok(path)
}
pub fn load_plugins(assets: &Assets) -> Result<Vec<RuntimePlugin>> {
    let root = assets.root.join("plugins");
    if !root.exists() {
        return Ok(vec![]);
    }
    if fs::symlink_metadata(&root).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(Error::invalid());
    }
    let mut plugins = Vec::new();
    for entry in fs::read_dir(&root).map_err(|_| Error::invalid())? {
        let entry = entry.map_err(|_| Error::invalid())?;
        if entry
            .file_type()
            .map_err(|_| Error::invalid())?
            .is_symlink()
        {
            return Err(Error::invalid());
        }
        if !entry.file_type().map_err(|_| Error::invalid())?.is_dir() {
            return Err(Error::new(
                crate::contracts::ErrorCode::UnsupportedSchema,
                "Plugins require a v2 directory package; legacy scripts were not converted",
            ));
        }
        plugins.push(load_package(&entry.path())?);
    }
    if plugins.len() > 64 {
        return Err(Error::invalid());
    }
    plugins.sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));
    Ok(plugins)
}
fn load_package(directory: &Path) -> Result<RuntimePlugin> {
    if fs::symlink_metadata(directory).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(Error::invalid());
    }
    let path = confined(directory, "webdeck.json")?;
    let value: Value =
        serde_json::from_slice(&read_bounded(&path)?).map_err(|_| Error::invalid())?;
    domain::validate("PluginManifest", &value)?;
    let manifest: PluginManifest = serde_json::from_value(value).map_err(|_| Error::invalid())?;
    if manifest.id.starts_with("builtin.")
        || directory.file_name().and_then(|v| v.to_str()) != Some(&manifest.id)
        || semver::Version::parse(&manifest.version).is_err()
    {
        return Err(Error::invalid());
    }
    let mut ids = std::collections::HashSet::new();
    if manifest.actions.iter().any(|a| !ids.insert(&a.id)) {
        return Err(Error::invalid());
    }
    let source_path = confined(directory, &manifest.entry)?;
    let bytes = read_bounded(&source_path)?;
    if format!("{:x}", Sha256::digest(&bytes)) != manifest.digest {
        return Err(Error::invalid());
    }
    let source = if manifest.backend == "sandbox_js" {
        if !manifest.entry.ends_with(".js") {
            return Err(Error::invalid());
        }
        Some(String::from_utf8(bytes).map_err(|_| Error::invalid())?)
    } else {
        let package: napi_vm_plugin_host::Manifest =
            serde_json::from_slice(&bytes).map_err(|_| Error::invalid())?;
        if !matches!(
            package.launch,
            napi_vm_plugin_host::Launch::Executable { .. }
        ) || package.id != manifest.id
            || package.version != manifest.version
            || manifest.contract.is_empty()
        {
            return Err(Error::invalid());
        }
        None
    };
    Ok(RuntimePlugin {
        manifest,
        root: directory.to_path_buf(),
        source,
    })
}
/// Startup quarantines invalid packages; explicit reload remains atomic and strict.
pub fn discover_plugins(assets: &Assets) -> Result<(Vec<RuntimePlugin>, usize)> {
    let root = assets.root.join("plugins");
    if !root.exists() {
        return Ok((vec![], 0));
    }
    if fs::symlink_metadata(&root).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(Error::invalid());
    }
    let mut plugins = Vec::new();
    let mut rejected = 0;
    for entry in fs::read_dir(root).map_err(|_| Error::invalid())? {
        let entry = entry.map_err(|_| Error::invalid())?;
        if plugins.len() + rejected >= 64 {
            return Err(Error::invalid());
        }
        match load_package(&entry.path()) {
            Ok(plugin) => plugins.push(plugin),
            Err(_) => rejected += 1,
        }
    }
    plugins.sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));
    Ok((plugins, rejected))
}
pub(super) fn validate_type(field: &PluginArgument, value: &Value) -> Result<()> {
    let valid = match field.r#type.as_str() {
        "string" => value.is_string(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "object" => value.is_object(),
        "array" => value.is_array(),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(Error::invalid())
    }
}

/// Embedded packages reserve their identifiers; disk packages cannot shadow them.
pub fn builtins() -> Vec<RuntimePlugin> {
    [
        ("builtin.obs", "obs", crate::contracts::Capability::Network),
        ("builtin.spotify", "spotify", crate::contracts::Capability::Network),
        ("builtin.http", "fetch", crate::contracts::Capability::Network),
        ("builtin.soundboard", "play_sound", crate::contracts::Capability::Audio),
    ].into_iter().map(|(id, command, capability)| {
        let source = format!("export function invoke_action(action,args,ctx) {{ if (action !== 'execute' || args.command.type !== '{command}') throw new Error('Invalid builtin action'); return ctx.invoke(args.command); }}");
        RuntimePlugin {
            manifest: PluginManifest {
                schema_version: 2, id: id.into(), version: "2.0.0".into(), entry: "index.js".into(), backend: "sandbox_js".into(), origin: "embedded".into(), contract: String::new(), digest: format!("{:x}", Sha256::digest(source.as_bytes())),
                actions: vec![crate::contracts::PluginAction {
                    id: "execute".into(), label: format!("Execute {command}"), capabilities: vec![capability],
                    arguments: std::collections::BTreeMap::from([("command".into(), crate::contracts::PluginArgument { r#type: "object".into(), required: true })]),
                    result: crate::contracts::PluginArgument { r#type: "object".into(), required: true },
                }],
            }, root: PathBuf::new(), source: Some(source),
        }
    }).collect()
}
