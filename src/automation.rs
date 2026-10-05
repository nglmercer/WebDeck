//! Data-only automation declarations shared by built-ins and plugin packages.
use crate::{
    contracts::*,
    domain::{self, Error, Result},
};
use serde_json::Value;
use std::collections::HashSet;

pub fn builtins() -> AutomationMetadata {
    serde_json::from_str(include_str!("../contracts/automation.json"))
        .expect("validated automation metadata")
}

pub fn valid_pointer(pointer: &str) -> bool {
    if !pointer.is_empty() && !pointer.starts_with('/') {
        return false;
    }
    let mut chars = pointer.chars();
    while let Some(c) = chars.next() {
        if c == '~' && !matches!(chars.next(), Some('0' | '1')) {
            return false;
        }
    }
    true
}

pub fn validate(metadata: &AutomationMetadata) -> Result<()> {
    domain::validate(
        "AutomationMetadata",
        &serde_json::to_value(metadata).map_err(|_| Error::invalid())?,
    )?;
    let mut ids = HashSet::new();
    for integration in &metadata.integrations {
        if !ids.insert(("integration", &integration.id)) {
            return Err(Error::invalid());
        }
        for pointer in integration
            .required_settings
            .iter()
            .chain(&integration.configuration)
        {
            if !valid_pointer(pointer) {
                return Err(Error::invalid());
            }
        }
        if let Some(asset) = &integration.authorization_asset {
            if asset.is_empty()
                || std::path::Path::new(asset)
                    .components()
                    .any(|c| !matches!(c, std::path::Component::Normal(_)))
            {
                return Err(Error::invalid());
            }
        }
        if let Some(probe) = &integration.probe {
            domain::validate_command(probe)?;
        }
        if integration
            .success
            .as_ref()
            .is_some_and(|p| !valid_pointer(&p.pointer))
        {
            return Err(Error::invalid());
        }
    }
    for recipe in &metadata.button_recipes {
        if !ids.insert(("recipe", &recipe.id)) {
            return Err(Error::invalid());
        }
        domain::validate_command(&recipe.discovery)?;
        domain::validate_command(&recipe.command)?;
        let template = serde_json::to_value(&recipe.command).map_err(|_| Error::invalid())?;
        for pointer in [
            &recipe.items_pointer,
            &recipe.identity_pointer,
            &recipe.label_pointer,
        ]
        .into_iter()
        .chain(recipe.bindings.values())
        {
            if !valid_pointer(pointer) {
                return Err(Error::invalid());
            }
        }
        for pointer in recipe.bindings.keys() {
            // Bind existing arguments only; command identity and plugin identity remain fixed.
            if !valid_pointer(pointer)
                || pointer.is_empty()
                || template.pointer(pointer).is_none()
                || matches!(
                    pointer.as_str(),
                    "/type" | "/plugin_id" | "/version" | "/action_id"
                )
            {
                return Err(Error::invalid());
            }
        }
    }
    for presentation in &metadata.presentations {
        if presentation.selector.is_empty() {
            return Err(Error::invalid());
        }
        for pointer in presentation.arguments.values() {
            if !valid_pointer(pointer) {
                return Err(Error::invalid());
            }
        }
        if presentation
            .result_pointer
            .as_ref()
            .is_some_and(|p| !valid_pointer(p))
        {
            return Err(Error::invalid());
        }
        if let Some(view) = &presentation.result_view {
            if !valid_pointer(&view.items_pointer)
                || view.columns.values().any(|p| !valid_pointer(p))
            {
                return Err(Error::invalid());
            }
        }
    }
    Ok(())
}

pub fn validate_plugin(manifest: &PluginManifest) -> Result<()> {
    let Some(metadata) = &manifest.automation else {
        return Ok(());
    };
    validate(metadata)?;
    let owns = |command: &Command| {
        matches!(command, Command::Plugin { plugin_id, version, action_id, .. }
        if plugin_id == &manifest.id && version == &manifest.version && manifest.actions.iter().any(|a| &a.id == action_id))
    };
    let prefix = format!("{}.", manifest.id);
    if metadata
        .integrations
        .iter()
        .any(|i| !i.id.starts_with(&prefix) || i.probe.as_ref().is_some_and(|c| !owns(c)))
        || metadata
            .button_recipes
            .iter()
            .any(|r| !r.id.starts_with(&prefix) || !owns(&r.discovery) || !owns(&r.command))
        || metadata.presentations.iter().any(|p| {
            p.selector.get("type") != Some(&Value::String("plugin".into()))
                || p.selector.get("plugin_id") != Some(&Value::String(manifest.id.clone()))
        })
    {
        return Err(Error::new(
            ErrorCode::InvalidInput,
            "Plugin automation must reference its own namespaced actions",
        ));
    }
    Ok(())
}

pub fn collect(plugins: &[PluginManifest], disabled: &[String]) -> Result<AutomationMetadata> {
    let mut metadata = builtins();
    for plugin in plugins.iter().filter(|p| !disabled.contains(&p.id)) {
        validate_plugin(plugin)?;
        if let Some(extra) = &plugin.automation {
            metadata.integrations.extend(extra.integrations.clone());
            metadata.button_recipes.extend(extra.button_recipes.clone());
            metadata.presentations.extend(extra.presentations.clone());
        }
    }
    validate(&metadata)?;
    Ok(metadata)
}

pub fn matches(selector: &std::collections::BTreeMap<String, Value>, command: &Value) -> bool {
    fn subset(expected: &Value, actual: &Value) -> bool {
        match expected.as_object() {
            Some(fields) => fields
                .iter()
                .all(|(key, value)| actual.get(key).is_some_and(|actual| subset(value, actual))),
            None => expected == actual,
        }
    }
    selector
        .iter()
        .all(|(key, value)| command.get(key).is_some_and(|actual| subset(value, actual)))
}

pub fn command_allowed(
    command: &Command,
    capabilities: &[Capability],
    plugins: &[PluginManifest],
) -> bool {
    if !capabilities.contains(&command.capability()) {
        return false;
    }
    if let Command::Plugin {
        plugin_id,
        action_id,
        ..
    } = command
    {
        return plugins
            .iter()
            .find(|p| &p.id == plugin_id)
            .and_then(|p| p.actions.iter().find(|a| &a.id == action_id))
            .is_some_and(|a| a.capabilities.iter().all(|c| capabilities.contains(c)));
    }
    true
}

/// Validate command availability and plugin argument types before planning writes.
pub fn validate_catalog_command(command: &Command, catalog: &CatalogResponse) -> Result<()> {
    domain::validate_command(command)?;
    if let Command::Plugin {
        plugin_id,
        version,
        action_id,
        args,
    } = command
    {
        let action = catalog
            .plugins
            .iter()
            .find(|p| &p.id == plugin_id && &p.version == version)
            .and_then(|p| p.actions.iter().find(|a| &a.id == action_id))
            .ok_or_else(Error::invalid)?;
        for (name, spec) in &action.arguments {
            match args.get(name) {
                None if spec.required => {
                    return Err(Error::new(
                        ErrorCode::InvalidInput,
                        format!("Missing plugin argument '{name}'"),
                    ))
                }
                Some(value) => {
                    crate::runtime::plugins::validate_type(spec, value).map_err(|_| {
                        Error::new(
                            ErrorCode::InvalidInput,
                            format!("Plugin argument '{name}' has the wrong type"),
                        )
                    })?;
                }
                None => {}
            }
        }
        if args.keys().any(|key| !action.arguments.contains_key(key)) {
            return Err(Error::new(
                ErrorCode::InvalidInput,
                "Unknown plugin argument",
            ));
        }
    } else {
        let value = serde_json::to_value(command).map_err(|_| Error::invalid())?;
        if !catalog.commands.iter().any(|c| value["type"] == c.id) {
            return Err(Error::new(
                ErrorCode::InvalidInput,
                "Command is unavailable in this catalog",
            ));
        }
    }
    Ok(())
}
