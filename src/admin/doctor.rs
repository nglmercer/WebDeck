use super::client::WebDeckAdminClient;
use super::error::{AdminError, ErrorInfo, ErrorKind, ExitCode, Result};
use super::output::Outcome;
use crate::domain;
use serde_json::{json, Value};

fn check(id: &str, level: &str, message: String) -> Value {
    json!({
        "id": id,
        "level": level,
        "ok": level == "pass",
        "message": message
    })
}

pub async fn status(client: &WebDeckAdminClient) -> Result<Outcome> {
    let version = client.version().await?;
    let boot = client.boot().await?;
    let runtime = client.runtime_status().await?;
    let integrations = client.integration_status().await?;
    let integration_rows = super::integrations::states(&integrations)
        .iter()
        .map(|(id, health)| format!("{id}: {}", super::integrations::state_label(health.state)))
        .collect::<Vec<_>>()
        .join(", ");
    let folders = boot.layout.folders.len();
    let buttons: usize = boot.layout.folders.iter().map(|f| f.buttons.len()).sum();
    let data = json!({
        "url": client.base_url(),
        "api_version": version.api_version,
        "server_version": version.version,
        "revision": boot.revision,
        "healthy": runtime.healthy,
        "can_edit": boot.can_edit,
        "capabilities": boot.capabilities,
        "folders": folders,
        "buttons": buttons,
        "plugins": {
            "total": runtime.plugins.len(),
            "loaded": runtime.loaded_plugins,
            "disabled": runtime.disabled_plugins
        },
        "integrations": {
            "obs": integrations.obs,
            "spotify": integrations.spotify,
            "checked_at": integrations.checked_at,
            "items": super::integrations::states(&integrations)
        }
    });
    let human = format!(
        "WebDeck {} at {}\nrevision {}, healthy: {}, edit: {}\nfolders: {folders}, buttons: {buttons}\nplugins: {} available, {} active sessions (loaded on first use), {} disabled\n{}",
        version.version,
        client.base_url(),
        boot.revision,
        runtime.healthy,
        boot.can_edit,
        runtime.plugins.len(),
        runtime.loaded_plugins.len(),
        runtime.disabled_plugins.len(),
        integration_rows
    );
    Ok(Outcome::ok(data, human))
}

pub async fn doctor(client: &WebDeckAdminClient) -> Result<Outcome> {
    let mut checks = Vec::new();
    let version = client.version().await?;
    checks.push(check(
        "server.reachable",
        "pass",
        format!("WebDeck {} responding", version.version),
    ));
    if version.api_version != 2 {
        checks.push(check(
            "server.api_version",
            "fail",
            format!("Expected API v2, server reports v{}", version.api_version),
        ));
    }
    let config = client.get_config().await?;
    checks.push(check(
        "admin.access",
        "pass",
        "Local administrator access confirmed".to_string(),
    ));
    if config.config.schema_version == 2 {
        checks.push(check(
            "schema.version",
            "pass",
            "Configuration schema version 2".to_string(),
        ));
    } else {
        checks.push(check(
            "schema.version",
            "fail",
            format!("Schema version {}", config.config.schema_version),
        ));
    }
    match domain::validate_config(&config.config) {
        Ok(()) => checks.push(check(
            "config.valid",
            "pass",
            "Configuration passes validation".to_string(),
        )),
        Err(e) => {
            let code = serde_json::to_value(e.code)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default();
            checks.push(check(
                "config.valid",
                "fail",
                format!("{code}: {}", e.message),
            ))
        }
    }
    match client.runtime_status().await {
        Ok(runtime) => {
            if runtime.healthy {
                checks.push(check(
                    "runtime.healthy",
                    "pass",
                    "Runtime is healthy".to_string(),
                ));
            } else {
                checks.push(check(
                    "runtime.healthy",
                    "fail",
                    "Runtime is unhealthy".to_string(),
                ));
            }
            let known: Vec<&String> = runtime.plugins.iter().map(|p| &p.id).collect();
            let missing: Vec<&String> = runtime
                .loaded_plugins
                .iter()
                .filter(|id| !known.contains(id))
                .collect();
            if missing.is_empty() {
                checks.push(check(
                    "plugins.consistent",
                    "pass",
                    "Loaded plugins match the package set".to_string(),
                ));
            } else {
                checks.push(check(
                    "plugins.consistent",
                    "warn",
                    format!(
                        "Loaded but not installed: {}",
                        missing
                            .iter()
                            .map(|s| s.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ));
            }
            if !runtime.disabled_plugins.is_empty() {
                checks.push(check(
                    "plugins.enabled",
                    "warn",
                    format!("Disabled plugins: {}", runtime.disabled_plugins.join(", ")),
                ));
            }
        }
        Err(e) => checks.push(check(
            "runtime.healthy",
            "fail",
            format!("Runtime status unavailable: {}", e.message()),
        )),
    }
    match client.integration_status().await {
        Ok(status) => {
            for (id, health) in super::integrations::states(&status) {
                let state = health.state;
                let level = match state {
                    crate::contracts::IntegrationState::Connected => "pass",
                    crate::contracts::IntegrationState::Failed => "warn",
                    _ => "warn",
                };
                checks.push(check(
                    &format!("integrations.{id}"),
                    level,
                    super::integrations::state_label(state),
                ));
            }
        }
        Err(e) => checks.push(check(
            "integrations.status",
            "fail",
            format!("Integration status unavailable: {}", e.message()),
        )),
    }
    let failures = checks.iter().filter(|c| c["level"] == "fail").count();
    let warnings = checks.iter().filter(|c| c["level"] == "warn").count();
    let ok = failures == 0;
    let mut human = String::new();
    for c in &checks {
        human.push_str(&format!(
            "[{}] {}: {}\n",
            c["level"].as_str().unwrap_or("?"),
            c["id"].as_str().unwrap_or("?"),
            c["message"].as_str().unwrap_or("")
        ));
    }
    human.push_str(&format!(
        "{} check(s): {} failure(s), {} warning(s)",
        checks.len(),
        failures,
        warnings
    ));
    let data = json!({"ok": ok, "checks": checks, "failures": failures, "warnings": warnings});
    if ok {
        Ok(Outcome::ok(data, human))
    } else {
        Ok(Outcome::failed(
            data,
            ErrorInfo {
                code: ErrorKind::Generic.code(),
                message: format!("{failures} health check(s) failed"),
                details: None,
            },
            ExitCode::Failure as i32,
            human,
        ))
    }
}

pub async fn capabilities(client: &WebDeckAdminClient) -> Result<Outcome> {
    let boot = client.boot().await?;
    let data = json!({
        "api_version": boot.api_version,
        "revision": boot.revision,
        "can_edit": boot.can_edit,
        "capabilities": boot.capabilities,
        "button_capabilities": boot.button_capabilities
    });
    let list = boot
        .capabilities
        .iter()
        .map(|c| format!("{:?}", c).to_lowercase())
        .collect::<Vec<_>>()
        .join(", ");
    Ok(Outcome::ok(
        data,
        format!(
            "revision {}, edit: {}, capabilities: {list}",
            boot.revision, boot.can_edit
        ),
    ))
}

pub fn schema(name: Option<&str>) -> Result<Outcome> {
    let schema = domain::schema();
    match name {
        None => Ok(Outcome::ok(
            json!({"schema": schema}),
            serde_json::to_string_pretty(schema)
                .map_err(|_| AdminError::new(ErrorKind::Generic, "Cannot encode schema"))?,
        )),
        Some(name) => {
            let definition = schema["$defs"].get(name).ok_or_else(|| {
                AdminError::invalid_arguments(format!("Unknown schema definition '{name}'"))
            })?;
            Ok(Outcome::ok(
                json!({"name": name, "schema": definition}),
                serde_json::to_string_pretty(definition)
                    .map_err(|_| AdminError::new(ErrorKind::Generic, "Cannot encode schema"))?,
            ))
        }
    }
}
