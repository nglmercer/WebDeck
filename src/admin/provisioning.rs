//! Revision-checked, atomic button provisioning from data-only catalog recipes.
use super::{
    client::{MutationReport, WebDeckAdminClient},
    error::{AdminError, ErrorKind, Result},
    output::Outcome,
    report_value,
};
use crate::{
    contracts::{Button, ButtonAction, ButtonRecipe, CatalogResponse, Command, Config, Folder},
    domain,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub fn plan(
    config: &Config,
    catalog: &CatalogResponse,
    recipe: &ButtonRecipe,
    result: &Value,
    folder_id: &str,
) -> Result<(Config, Vec<Value>)> {
    let items = result
        .pointer(&recipe.items_pointer)
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AdminError::new(
                ErrorKind::Integration,
                "Discovery result does not contain the recipe's item array",
            )
        })?;
    let mut config = config.clone();
    let index = match config.layout.folders.iter().position(|f| f.id == folder_id) {
        Some(index) => index,
        None => {
            config.layout.folders.push(Folder {
                id: folder_id.into(),
                label: recipe.folder_label.clone(),
                buttons: Vec::new(),
                extensions: Default::default(),
            });
            config.layout.folders.len() - 1
        }
    };
    let mut seen = std::collections::HashSet::new();
    let mut created = Vec::new();
    for item in items {
        let identity = item
            .pointer(&recipe.identity_pointer)
            .filter(|v| !v.is_null())
            .ok_or_else(|| {
                AdminError::new(
                    ErrorKind::Validation,
                    "Discovery item has no stable identity",
                )
            })?;
        let identity = serde_json::to_string(identity)
            .map_err(|_| AdminError::new(ErrorKind::Validation, "Cannot encode item identity"))?;
        if !seen.insert(identity.clone()) {
            return Err(AdminError::new(
                ErrorKind::Validation,
                "Discovery returned duplicate item identities",
            ));
        }
        let label = item
            .pointer(&recipe.label_pointer)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                AdminError::new(
                    ErrorKind::Validation,
                    "Discovery item has no non-empty label",
                )
            })?;
        let mut command = serde_json::to_value(&recipe.command)
            .map_err(|_| AdminError::new(ErrorKind::Validation, "Invalid recipe command"))?;
        for (destination, source) in &recipe.bindings {
            let value = item.pointer(source).ok_or_else(|| {
                AdminError::with_details(
                    ErrorKind::Validation,
                    "Discovery item is missing a bound value",
                    json!({"pointer":source}),
                )
            })?;
            let target = command.pointer_mut(destination).ok_or_else(|| {
                AdminError::new(
                    ErrorKind::Validation,
                    "Recipe binding does not select a command argument",
                )
            })?;
            *target = value.clone();
        }
        let command: Command = serde_json::from_value(command).map_err(|_| {
            AdminError::new(
                ErrorKind::Validation,
                "Bound command does not match its schema",
            )
        })?;
        crate::automation::validate_catalog_command(&command, catalog)
            .map_err(AdminError::from_domain)?;
        let action = ButtonAction::Command { command };
        if config.layout.folders[index]
            .buttons
            .iter()
            .any(|b| b.action == action)
        {
            continue;
        }
        let digest = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&(folder_id, &recipe.id, identity))
                    .expect("serializable identity")
            )
        );
        let base = format!("generated-{digest}");
        let mut id = base.clone();
        let mut suffix = 0;
        while config
            .layout
            .folders
            .iter()
            .any(|f| f.id == id || f.buttons.iter().any(|b| b.id == id))
        {
            suffix += 1;
            id = format!("{base}-{suffix}");
        }
        config.layout.folders[index].buttons.push(Button {
            id: id.clone(),
            label: label.into(),
            icon: String::new(),
            color: String::new(),
            action,
            extensions: Default::default(),
        });
        created.push(json!({"id": id, "label": label}));
    }
    domain::validate_config(&config).map_err(AdminError::from_domain)?;
    Ok((config, created))
}

pub async fn recipes(client: &WebDeckAdminClient) -> Result<Outcome> {
    let metadata = client.catalog().await?.automation.ok_or_else(|| {
        AdminError::new(
            ErrorKind::NotFound,
            "Server does not publish automation metadata",
        )
    })?;
    let rows = metadata
        .button_recipes
        .iter()
        .map(|r| format!("{}\t{}", r.id, r.label))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(Outcome::ok(
        json!({"recipes":metadata.button_recipes}),
        rows,
    ))
}

pub async fn ensure(
    client: &WebDeckAdminClient,
    recipe_id: &str,
    folder_id: &str,
    revision: Option<u64>,
    dry_run: bool,
) -> Result<Outcome> {
    let catalog = client.catalog().await?;
    let recipe = catalog
        .automation
        .as_ref()
        .and_then(|m| m.button_recipes.iter().find(|r| r.id == recipe_id))
        .ok_or_else(|| {
            AdminError::with_details(
                ErrorKind::NotFound,
                "Unknown button recipe",
                json!({"id":recipe_id}),
            )
        })?;
    let current = client.get_config().await?;
    client.ensure_revision(revision, current.revision)?;
    crate::automation::validate_catalog_command(&recipe.discovery, &catalog)
        .map_err(AdminError::from_domain)?;
    let result = client.run_command(recipe.discovery.clone()).await?;
    let (config, created) = plan(&current.config, &catalog, recipe, &result, folder_id)?;
    let changed = config != current.config;
    let report = if !changed {
        MutationReport {
            operation: "button_generate".into(),
            revision: current.revision,
            previous_revision: None,
            applied: false,
            dry_run,
        }
    } else if dry_run {
        MutationReport::planned("button_generate", current.revision)
    } else {
        let response = match client.put_config(current.revision, config).await {
            Ok(response) => response,
            Err(error) => return Err(client.enrich_conflict(error, current.revision).await),
        };
        MutationReport::applied("button_generate", current.revision, response.revision)
    };
    let human = if !changed {
        format!(
            "{} already ensured in '{folder_id}' at revision {}.",
            recipe.label, report.revision
        )
    } else {
        format!(
            "{} {} button(s) in '{folder_id}' at revision {}.",
            if dry_run {
                "Dry run: would create"
            } else {
                "Created"
            },
            created.len(),
            report.revision
        )
    };
    Ok(Outcome::ok(
        report_value(
            &report,
            json!({"recipe": recipe_id, "folder": folder_id, "created": created}),
        ),
        human,
    ))
}
