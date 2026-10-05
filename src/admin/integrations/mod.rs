pub mod obs;

use super::client::WebDeckAdminClient;
use super::error::Result;
use super::output::Outcome;
use serde_json::json;

fn state_label(state: crate::contracts::IntegrationState) -> String {
    format!("{:?}", state).to_lowercase()
}

pub async fn status(client: &WebDeckAdminClient) -> Result<Outcome> {
    let status = client.integration_status().await?;
    Ok(Outcome::ok(
        json!({
            "api_version": status.api_version,
            "obs": status.obs,
            "spotify": status.spotify,
            "checked_at": status.checked_at
        }),
        format!(
            "obs: {}, spotify: {} (checked at {})",
            state_label(status.obs),
            state_label(status.spotify),
            status.checked_at
        ),
    ))
}
