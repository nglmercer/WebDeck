//! Host-side declarative OBS projects. All project paths are read locally by the CLI.
mod planner;
mod prune;
mod sync;
mod template;
use super::{AdminError, ErrorKind, Outcome, Result};
pub use planner::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashSet},
    io::Read,
    path::{Path, PathBuf},
};
pub use sync::*;
pub use template::*;
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u32,
    pub project_id: String,
    pub collection: String,
    pub scenes: Vec<String>,
    #[serde(default)]
    pub shared: Vec<String>,
    pub webdeck: Option<Deck>,
    pub sync: Option<Sync>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Deck {
    pub folder_id: String,
    pub folder_label: Option<String>,
    #[serde(default)]
    pub generate_buttons: bool,
    #[serde(default)]
    pub sync_managed_buttons: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Sync {
    #[serde(default = "merge")]
    pub mode: String,
    #[serde(default)]
    pub delete_unmanaged: bool,
}
fn merge() -> String {
    "merge".into()
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Scene {
    pub id: String,
    pub name: String,
    pub items: Vec<Item>,
    pub instance_properties: Option<InstanceProperties>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceProperties {
    pub enabled: Option<bool>,
    pub transform: Option<Value>,
    pub index: Option<u64>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Item {
    pub id: String,
    pub kind: String,
    pub input_name: Option<String>,
    pub input_id: Option<String>,
    pub input_kind: Option<String>,
    pub scene_id: Option<String>,
    pub settings: Option<Value>,
    pub enabled: Option<bool>,
    pub transform: Option<Value>,
    pub index: Option<u64>,
}
pub struct Project {
    pub root: PathBuf,
    pub manifest: Manifest,
    pub scenes: Vec<Scene>,
}
pub fn schema() -> Value {
    serde_json::from_str(include_str!("../../../contracts/obs-project.schema.json"))
        .expect("project schema")
}
fn invalid(file: &str, pointer: &str, code: &str, message: &str) -> AdminError {
    AdminError::with_details(
        ErrorKind::Validation,
        message,
        json!({"file":file,"pointer":pointer,"code":code}),
    )
}
fn validate(
    root: &Value,
    schema: &Value,
    value: &Value,
    file: &str,
    pointer: &str,
    depth: usize,
) -> Result<()> {
    let fail = || {
        invalid(
            file,
            pointer,
            "schema",
            "Value does not match OBS project schema",
        )
    };
    if depth > 48 {
        return Err(fail());
    }
    if let Some(reference) = schema["$ref"].as_str() {
        return validate(
            root,
            root.pointer(&reference[1..]).ok_or_else(fail)?,
            value,
            file,
            pointer,
            depth + 1,
        );
    }
    if schema.get("const").is_some_and(|v| v != value)
        || schema["enum"]
            .as_array()
            .is_some_and(|v| !v.contains(value))
    {
        return Err(fail());
    }
    let bound = |n: f64, low: &str, high: &str| -> Result<()> {
        if schema[low].as_f64().is_some_and(|v| n < v)
            || schema[high].as_f64().is_some_and(|v| n > v)
        {
            Err(fail())
        } else {
            Ok(())
        }
    };
    match schema["type"].as_str() {
        Some("object") => {
            let object = value.as_object().ok_or_else(fail)?;
            if let Some(required) = schema["required"].as_array() {
                for name in required {
                    if !object.contains_key(name.as_str().unwrap_or("")) {
                        return Err(fail());
                    }
                }
            }
            for (key, child) in object {
                let child_pointer =
                    format!("{pointer}/{}", key.replace('~', "~0").replace('/', "~1"));
                if let Some(field) = schema["properties"].get(key) {
                    validate(root, field, child, file, &child_pointer, depth + 1)?;
                } else if schema["additionalProperties"] == false {
                    return Err(invalid(
                        file,
                        &child_pointer,
                        "unknown_field",
                        "Unknown project field",
                    ));
                }
            }
        }
        Some("array") => {
            let array = value.as_array().ok_or_else(fail)?;
            bound(array.len() as f64, "minItems", "maxItems")?;
            for (index, item) in array.iter().enumerate() {
                validate(
                    root,
                    &schema["items"],
                    item,
                    file,
                    &format!("{pointer}/{index}"),
                    depth + 1,
                )?;
            }
        }
        Some("string") => {
            let string = value.as_str().ok_or_else(fail)?;
            if string.chars().any(char::is_control) {
                return Err(fail());
            }
            bound(string.chars().count() as f64, "minLength", "maxLength")?;
            if let Some(pattern) = schema["pattern"].as_str() {
                if !regex::Regex::new(pattern)
                    .map_err(|_| fail())?
                    .is_match(string)
                {
                    return Err(fail());
                }
            }
        }
        Some("number" | "integer") => {
            let number = value.as_f64().ok_or_else(fail)?;
            if schema["type"] == "integer" && number.fract() != 0.0 {
                return Err(fail());
            }
            bound(number, "minimum", "maximum")?;
        }
        Some("boolean") if !value.is_boolean() => return Err(fail()),
        _ => {}
    }
    Ok(())
}
fn read(root: &Path, name: &str) -> Result<Value> {
    read_limit(root, name, 1_048_576)
}
fn read_limit(root: &Path, name: &str, limit: usize) -> Result<Value> {
    let relative = Path::new(name);
    if relative
        .components()
        .any(|c| !matches!(c, std::path::Component::Normal(_)))
        || relative.is_absolute()
    {
        return Err(invalid(
            name,
            "",
            "path_escape",
            "Only project-relative paths are allowed",
        ));
    }
    let path = root.join(relative).canonicalize().map_err(|_| {
        invalid(
            name,
            "",
            "missing_file",
            "Cannot read referenced project file",
        )
    })?;
    if !path.starts_with(root) {
        return Err(invalid(
            name,
            "",
            "symlink_escape",
            "Project path escapes its root",
        ));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .and_then(|file| file.take(limit as u64 + 1).read_to_end(&mut bytes))
        .map_err(|_| invalid(name, "", "read_failed", "Cannot read project file"))?;
    if bytes.len() > limit {
        return Err(invalid(
            name,
            "",
            "size_limit",
            "File exceeds its bounded size limit",
        ));
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| invalid(name, "", "invalid_json", "Invalid project JSON"))
}
pub fn load(path: &Path) -> Result<Project> {
    let root = path.canonicalize().map_err(|_| {
        invalid(
            "project.json",
            "",
            "missing_root",
            "Project folder does not exist",
        )
    })?;
    let schema = schema();
    let value = read(&root, "project.json")?;
    validate(&schema, &schema, &value, "project.json", "", 0)?;
    let manifest: Manifest = serde_json::from_value(value)
        .map_err(|_| invalid("project.json", "", "schema", "Invalid manifest"))?;
    let mut scenes = Vec::new();
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    for file in manifest.shared.iter().chain(&manifest.scenes) {
        let value = read(&root, file)?;
        validate(&schema, &schema["$defs"]["Scene"], &value, file, "", 0)?;
        let scene: Scene = serde_json::from_value(value)
            .map_err(|_| invalid(file, "", "schema", "Invalid scene"))?;
        if !ids.insert(scene.id.clone()) || !names.insert(scene.name.clone()) {
            return Err(invalid(
                file,
                "/id",
                "duplicate_scene",
                "Scene IDs and names must be unique",
            ));
        }
        let mut items = HashSet::new();
        let mut sources = HashSet::new();
        let mut indices = HashSet::new();
        for (index, item) in scene.items.iter().enumerate() {
            if !items.insert(&item.id) {
                return Err(invalid(
                    file,
                    &format!("/items/{index}/id"),
                    "duplicate_item",
                    "Scene item IDs must be unique",
                ));
            }
            if item.index.is_some_and(|index| !indices.insert(index)) {
                return Err(invalid(
                    file,
                    &format!("/items/{index}/index"),
                    "duplicate_index",
                    "Declared scene item order indices must be unique",
                ));
            }
            let valid = match item.kind.as_str() {
                "scene_reference" => {
                    item.scene_id.is_some()
                        && item.input_id.is_none()
                        && item.input_name.is_none()
                        && item.input_kind.is_none()
                        && item.settings.is_none()
                }
                "existing_input" => {
                    item.input_name.is_some()
                        && item.input_kind.is_none()
                        && item.scene_id.is_none()
                        && item.settings.is_none()
                }
                "input" => {
                    item.input_name.is_some()
                        && item.input_kind.is_some()
                        && item.scene_id.is_none()
                }
                _ => false,
            };
            if !valid {
                return Err(invalid(file,&format!("/items/{index}"),"invalid_source","Input requires inputName/inputKind; scene references require sceneId; existing inputs cannot declare settings"));
            }
            let source = item
                .scene_id
                .as_ref()
                .or(item.input_name.as_ref())
                .expect("validated");
            if !sources.insert(source) {
                return Err(invalid(
                    file,
                    &format!("/items/{index}"),
                    "duplicate_source",
                    "Use one managed instance of each source per scene",
                ));
            }
            if let Some(settings) = &item.settings {
                check_settings(settings, file, &format!("/items/{index}/settings"), 0)?;
            }
        }
        scenes.push(scene);
    }
    if scenes.iter().map(|scene| scene.items.len()).sum::<usize>() > 2048 {
        return Err(invalid(
            "project.json",
            "/scenes",
            "graph_limit",
            "Project exceeds 2048 scene items",
        ));
    }
    let mut declared_inputs = BTreeMap::new();
    let mut input_names = BTreeMap::new();
    for scene in &scenes {
        for item in &scene.items {
            if item.kind == "input" {
                let name = item.input_name.as_deref().expect("validated");
                let id = item.input_id.as_deref().unwrap_or(name);
                if names.contains(name) {
                    return Err(invalid(
                        "project.json",
                        "/scenes",
                        "source_name_collision",
                        "Input names and scene names share the OBS source namespace",
                    ));
                }
                let declaration = (name, item.input_kind.as_deref(), item.settings.as_ref());
                if declared_inputs
                    .insert(id, declaration)
                    .is_some_and(|previous| previous != declaration)
                {
                    return Err(invalid(
                        "project.json",
                        "/scenes",
                        "input_definition_collision",
                        "Repeated shared input declarations must agree",
                    ));
                }
                if input_names
                    .insert(name, id)
                    .is_some_and(|previous| previous != id)
                {
                    return Err(invalid(
                        "project.json",
                        "/scenes",
                        "input_identity_collision",
                        "One input name cannot have multiple logical input IDs",
                    ));
                }
            }
        }
    }
    let by_id: BTreeMap<_, _> = scenes.iter().map(|s| (s.id.as_str(), s)).collect();
    fn visit<'a>(
        id: &'a str,
        scenes: &BTreeMap<&'a str, &'a Scene>,
        active: &mut HashSet<&'a str>,
        done: &mut HashSet<&'a str>,
    ) -> Result<()> {
        if done.contains(id) {
            return Ok(());
        }
        if !active.insert(id) {
            return Err(invalid(
                "project.json",
                "/scenes",
                "scene_cycle",
                "Nested scene references form a cycle",
            ));
        }
        let scene = scenes.get(id).ok_or_else(|| {
            invalid(
                "project.json",
                "/scenes",
                "missing_scene",
                "sceneId is not declared",
            )
        })?;
        for item in &scene.items {
            if let Some(child) = item.scene_id.as_deref() {
                visit(child, scenes, active, done)?;
            }
        }
        active.remove(id);
        done.insert(id);
        Ok(())
    }
    let mut done = HashSet::new();
    for scene in &scenes {
        visit(&scene.id, &by_id, &mut HashSet::new(), &mut done)?;
    }
    Ok(Project {
        root,
        manifest,
        scenes,
    })
}
fn check_settings(value: &Value, file: &str, pointer: &str, depth: usize) -> Result<()> {
    if let Some(text) = value.as_str() {
        if let Ok(url) = reqwest::Url::parse(text) {
            if !url.username().is_empty() || url.password().is_some() || url.scheme() == "file" {
                return Err(invalid(
                    file,
                    pointer,
                    "unsafe_url",
                    "Credential URLs and local file URLs are not portable project settings",
                ));
            }
        }
    }
    if depth == 0 && value.to_string().len() > 32768 {
        return Err(invalid(
            file,
            pointer,
            "settings_size",
            "Input settings exceed 32 KiB",
        ));
    }
    if depth > 24 {
        return Err(invalid(
            file,
            pointer,
            "settings_depth",
            "Settings nesting exceeds limit",
        ));
    }
    if let Some(map) = value.as_object() {
        for (key, value) in map {
            if ["password", "streamkey", "token", "secret"]
                .iter()
                .any(|part| key.to_lowercase().contains(part))
            {
                return Err(invalid(
                    file,
                    pointer,
                    "secret_field",
                    "Keep credentials out of OBS projects",
                ));
            }
            if matches!(key.as_str(), "file" | "local_file" | "path") {
                return Err(invalid(file,pointer,"asset_unsupported","Local asset settings require a host asset binding; use an existing input until asset upload is supported"));
            }
            check_settings(value, file, pointer, depth + 1)?;
        }
    }
    if let Some(array) = value.as_array() {
        for value in array {
            check_settings(value, file, pointer, depth + 1)?;
        }
    }
    Ok(())
}
pub fn validate_project(path: &Path) -> Result<Outcome> {
    let project = load(path)?;
    Ok(Outcome::ok(
        json!({"valid":true,"projectId":project.manifest.project_id,"scenes":project.scenes.len()}),
        "OBS project is valid offline.",
    ))
}
pub fn init(path: &Path) -> Result<Outcome> {
    if path.exists() {
        return Err(AdminError::invalid_arguments(
            "Project init requires a new directory; existing files are never overwritten",
        ));
    }
    std::fs::create_dir(path).map_err(|_| {
        AdminError::new(
            ErrorKind::Generic,
            "Cannot create new project directory (parent must exist)",
        )
    })?;
    std::fs::create_dir(path.join("scenes"))
        .and_then(|_| std::fs::create_dir(path.join("shared")))
        .map_err(|_| AdminError::new(ErrorKind::Generic, "Cannot create project directory"))?;
    for (name, content) in [
        (
            "project.json",
            include_str!("../../../examples/obs-project/project.json"),
        ),
        (
            "scenes/starting.json",
            include_str!("../../../examples/obs-project/scenes/starting.json"),
        ),
        (
            "scenes/gameplay.json",
            include_str!("../../../examples/obs-project/scenes/gameplay.json"),
        ),
        (
            "scenes/brb.json",
            include_str!("../../../examples/obs-project/scenes/brb.json"),
        ),
        (
            "scenes/ending.json",
            include_str!("../../../examples/obs-project/scenes/ending.json"),
        ),
        (
            "shared/common-overlay.json",
            include_str!("../../../examples/obs-project/shared/common-overlay.json"),
        ),
    ] {
        crate::storage::protected_write(&path.join(name), content.as_bytes())
            .map_err(AdminError::from_domain)?;
    }
    validate_project(path)
}

#[cfg(test)]
mod tests;
