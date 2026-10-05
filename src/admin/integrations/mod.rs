pub mod obs;

use super::client::WebDeckAdminClient;
use super::error::{ErrorInfo, ErrorKind, ExitCode, Result};
use super::output::Outcome;
use crate::contracts::{IntegrationHealth, IntegrationState, IntegrationStatus};
use serde_json::json;
use std::collections::BTreeMap;

pub fn state_label(state: IntegrationState) -> String {
    serde_json::to_value(state)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

pub fn states(status: &IntegrationStatus) -> BTreeMap<String, IntegrationHealth> {
    status.integrations.clone().unwrap_or_else(|| {
        BTreeMap::from([
            (
                "obs".into(),
                IntegrationHealth {
                    state: status.obs,
                    checked_at: status.checked_at,
                },
            ),
            (
                "spotify".into(),
                IntegrationHealth {
                    state: status.spotify,
                    checked_at: 0,
                },
            ),
        ])
    })
}

pub async fn status(client: &WebDeckAdminClient) -> Result<Outcome> {
    let status = client.integration_status().await?;
    let rows = states(&status)
        .iter()
        .map(|(id, health)| {
            format!(
                "{id}: {} (last check: {})",
                state_label(health.state),
                health.checked_at
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(Outcome::ok(
        serde_json::to_value(status).expect("serializable status"),
        rows,
    ))
}

pub async fn list(client: &WebDeckAdminClient) -> Result<Outcome> {
    let metadata = client.catalog().await?.automation;
    let definitions = metadata.map(|m| m.integrations).unwrap_or_default();
    let rows = definitions
        .iter()
        .map(|d| format!("{}\t{}\tprobe: {}", d.id, d.label, d.probe.is_some()))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(Outcome::ok(json!({"integrations": definitions}), rows))
}

pub async fn check(client: &WebDeckAdminClient, id: &str) -> Result<Outcome> {
    let status = client.integration_check(id).await?;
    let health = states(&status).get(id).cloned();
    let failed = health
        .as_ref()
        .is_some_and(|h| h.state == IntegrationState::Failed);
    let human = format!(
        "{id}: {}",
        health
            .as_ref()
            .map(|h| state_label(h.state))
            .unwrap_or_else(|| "unavailable".into())
    );
    let data = json!({"id": id, "health": health, "integrations": states(&status)});
    if failed {
        Ok(Outcome::failed(
            data,
            ErrorInfo {
                code: ErrorKind::Integration.code(),
                message: format!("Integration '{id}' check failed"),
                details: None,
            },
            ExitCode::Integration as i32,
            human,
        ))
    } else {
        Ok(Outcome::ok(data, human))
    }
}
