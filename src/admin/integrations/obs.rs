use super::super::client::{MutationReport, WebDeckAdminClient};
use super::super::error::{AdminError, ErrorInfo, ErrorKind, ExitCode, Result};
use super::super::output::Outcome;
use super::super::report_value;
use crate::contracts::{Command, IntegrationState, Settings};
use crate::domain;
use serde_json::{json, Value};

pub struct ObsService<'a> {
    pub(super) client: &'a WebDeckAdminClient,
}

impl WebDeckAdminClient {
    pub fn obs(&self) -> ObsService<'_> {
        ObsService { client: self }
    }
}

fn password_from_stdin() -> Result<String> {
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .map_err(|e| AdminError::invalid_arguments(format!("Cannot read password: {e}")))?;
    Ok(line.trim_end_matches(['\r', '\n']).to_string())
}

fn scene_names(result: &Value) -> Vec<String> {
    result["scenes"]
        .as_array()
        .map(|scenes| {
            scenes
                .iter()
                .filter_map(|s| s["sceneName"].as_str())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn input_names(result: &Value) -> Vec<String> {
    result["inputs"]
        .as_array()
        .map(|inputs| {
            inputs
                .iter()
                .filter_map(|i| i["inputName"].as_str())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

impl<'a> ObsService<'a> {
    pub async fn status(&self) -> Result<Outcome> {
        let status = self.client.integration_status().await?;
        let label = super::state_label(status.obs);
        Ok(Outcome::ok(
            json!({
                "obs": status.obs,
                "checked_at": super::states(&status).get("obs").map(|h| h.checked_at).unwrap_or(0),
                "configured": status.obs != IntegrationState::NotConfigured
            }),
            format!("OBS integration state: {label}."),
        ))
    }

    pub async fn check(&self) -> Result<Outcome> {
        let status = self.client.obs_check().await?;
        let label = super::state_label(status.obs);
        let data = json!({
            "obs": status.obs,
            "checked_at": super::states(&status).get("obs").map(|h| h.checked_at).unwrap_or(0),
            "configured": status.obs != IntegrationState::NotConfigured
        });
        if status.obs == IntegrationState::Failed {
            return Ok(Outcome::failed(
                data,
                ErrorInfo {
                    code: ErrorKind::Integration.code(),
                    message: "OBS connection check failed".into(),
                    details: Some(json!({"obs": status.obs})),
                },
                ExitCode::Integration as i32,
                "OBS connection check failed.".to_string(),
            ));
        }
        Ok(Outcome::ok(data, format!("OBS connection check: {label}.")))
    }

    pub async fn configure(
        &self,
        host: Option<&str>,
        port: Option<u64>,
        password_stdin: bool,
        revision: Option<u64>,
        dry_run: bool,
    ) -> Result<Outcome> {
        if host.is_none() && port.is_none() && !password_stdin {
            return Err(AdminError::invalid_arguments(
                "obs configure requires --host, --port, and/or --password-stdin",
            ));
        }
        if let Some(port) = port {
            if !(1..=65535).contains(&port) {
                return Err(AdminError::invalid_arguments(
                    "OBS port must be between 1 and 65535",
                ));
            }
        }
        if let Some(host) = host {
            if host.trim().is_empty() {
                return Err(AdminError::invalid_arguments("OBS host cannot be empty"));
            }
        }
        let current = self.client.get_config().await?;
        if let Some(expected) = revision {
            self.client
                .ensure_revision(Some(expected), current.revision)?;
        }
        let expected = revision.unwrap_or(current.revision);
        let mut settings: Settings = current.config.settings.clone();
        if let Some(host) = host {
            settings.obs.host = host.trim().to_string();
        }
        if let Some(port) = port {
            settings.obs.port = port;
        }
        if password_stdin {
            settings.obs.password = password_from_stdin()?;
        }
        domain::validate(
            "Settings",
            &serde_json::to_value(&settings).unwrap_or(Value::Null),
        )
        .map_err(AdminError::from_domain)?;
        let obs = json!({
            "host": settings.obs.host,
            "port": settings.obs.port,
            "password_set": !settings.obs.password.is_empty()
        });
        if settings == current.config.settings {
            return Ok(Outcome::ok(
                report_value(
                    &MutationReport {
                        operation: "obs_configure".into(),
                        revision: current.revision,
                        previous_revision: None,
                        applied: false,
                        dry_run,
                    },
                    json!({"obs": obs}),
                ),
                format!(
                    "OBS settings unchanged (host {}, port {}).",
                    settings.obs.host, settings.obs.port
                ),
            ));
        }
        if dry_run {
            return Ok(Outcome::ok(
                report_value(
                    &MutationReport::planned("obs_configure", current.revision),
                    json!({"obs": obs}),
                ),
                format!(
                    "Dry run: would set OBS host {} port {} password={} at revision {}.",
                    settings.obs.host,
                    settings.obs.port,
                    if settings.obs.password.is_empty() {
                        "unset"
                    } else {
                        "set"
                    },
                    current.revision
                ),
            ));
        }
        let response = match self.client.put_settings(expected, settings).await {
            Ok(response) => response,
            Err(error) => return Err(self.client.enrich_conflict(error, expected).await),
        };
        Ok(Outcome::ok(
            report_value(
                &MutationReport::applied("obs_configure", expected, response.revision),
                json!({"obs": obs}),
            ),
            format!(
                "OBS settings updated (revision {expected} -> {}). host {}, port {}, password {}.",
                response.revision,
                obs["host"].as_str().unwrap_or(""),
                obs["port"],
                if obs["password_set"] == json!(true) {
                    "set"
                } else {
                    "unset"
                }
            ),
        ))
    }

    async fn action(&self, action: &str, target: &str) -> Result<(Value, Value)> {
        let command = Command::Obs {
            action: action.to_string(),
            target: target.to_string(),
        };
        let result = self.client.run_command(command).await?;
        Ok((json!({"action": action, "target": target}), result))
    }

    pub async fn scenes(&self) -> Result<Outcome> {
        let (_, result) = self.action("get_scenes", "").await?;
        let names = scene_names(&result);
        let human = if names.is_empty() {
            serde_json::to_string_pretty(&result)
                .map_err(|_| AdminError::new(ErrorKind::Generic, "Cannot encode result"))?
        } else {
            names.join(", ")
        };
        Ok(Outcome::ok(
            json!({"action": "get_scenes", "state": "completed", "result": result}),
            human,
        ))
    }

    pub async fn ensure_buttons(
        &self,
        folder_id: &str,
        revision: Option<u64>,
        dry_run: bool,
    ) -> Result<Outcome> {
        super::super::provisioning::ensure(self.client, "obs-scenes", folder_id, revision, dry_run)
            .await
    }

    pub async fn current_scene(&self) -> Result<Outcome> {
        let (_, result) = self.action("get_current_scene", "").await?;
        let human = result["currentProgramSceneName"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| {
                serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
            });
        Ok(Outcome::ok(
            json!({"action": "get_current_scene", "state": "completed", "result": result}),
            human,
        ))
    }

    pub async fn inputs(&self) -> Result<Outcome> {
        let (_, result) = self.action("get_inputs", "").await?;
        let names = input_names(&result);
        let human = if names.is_empty() {
            serde_json::to_string_pretty(&result)
                .map_err(|_| AdminError::new(ErrorKind::Generic, "Cannot encode result"))?
        } else {
            names.join(", ")
        };
        Ok(Outcome::ok(
            json!({"action": "get_inputs", "state": "completed", "result": result}),
            human,
        ))
    }

    pub async fn hotkeys(&self, target: Option<&str>) -> Result<Outcome> {
        match target {
            Some(key) if !key.is_empty() => {
                let (_, result) = self.action("hotkey", key).await?;
                Ok(Outcome::ok(
                    json!({"action": "hotkey", "target": key, "state": "completed", "result": result}),
                    format!("Triggered hotkey '{key}'."),
                ))
            }
            _ => {
                let (_, result) = self.action("get_hotkeys", "").await?;
                let human = serde_json::to_string_pretty(&result)
                    .map_err(|_| AdminError::new(ErrorKind::Generic, "Cannot encode result"))?;
                Ok(Outcome::ok(
                    json!({"action": "get_hotkeys", "state": "completed", "result": result}),
                    human,
                ))
            }
        }
    }

    async fn verb(&self, group: &str, verb: &str) -> Result<Outcome> {
        let action = match (group, verb) {
            ("stream", "status") => "get_stream_status",
            ("stream", "start") => "start_stream",
            ("stream", "stop") => "stop_stream",
            ("stream", "toggle") => "toggle_stream",
            ("recording", "status") => "get_recording_status",
            ("recording", "start") => "start_recording",
            ("recording", "stop") => "stop_recording",
            ("recording", "toggle") => "toggle_recording",
            ("recording", "pause") => "pause_recording",
            ("recording", "resume") => "resume_recording",
            ("virtual-camera", "status") => "get_virtual_camera_status",
            ("virtual-camera", "start") => "start_virtual_camera",
            ("virtual-camera", "stop") => "stop_virtual_camera",
            ("virtual-camera", "toggle") => "toggle_virtual_camera",
            _ => {
                return Err(AdminError::invalid_arguments(format!(
                    "Unknown {group} action '{verb}'"
                )))
            }
        };
        let (_, result) = self.action(action, "").await?;
        let pretty = serde_json::to_string_pretty(&result)
            .map_err(|_| AdminError::new(ErrorKind::Generic, "Cannot encode result"))?;
        let human = if verb == "status" {
            pretty
        } else {
            format!("OBS {group} {verb} requested.")
        };
        Ok(Outcome::ok(
            json!({"action": action, "state": "completed", "result": result}),
            human,
        ))
    }

    pub async fn stream(&self, verb: &str) -> Result<Outcome> {
        self.verb("stream", verb).await
    }

    pub async fn recording(&self, verb: &str) -> Result<Outcome> {
        self.verb("recording", verb).await
    }

    pub async fn virtual_camera(&self, verb: &str) -> Result<Outcome> {
        self.verb("virtual-camera", verb).await
    }

    pub async fn actions(&self) -> Result<Outcome> {
        let catalog = self.client.catalog().await?;
        let entry = catalog
            .commands
            .iter()
            .find(|c| c.id == "obs")
            .ok_or_else(|| {
                AdminError::with_details(
                    ErrorKind::NotFound,
                    "OBS actions are unavailable on this server",
                    json!({"id": "obs"}),
                )
            })?;
        let actions = entry
            .schema
            .get("properties")
            .and_then(|p| p.get("action"))
            .and_then(|a| a.get("enum"))
            .and_then(|e| e.as_array())
            .cloned()
            .unwrap_or_default();
        let count = actions.len();
        let human = actions
            .iter()
            .filter_map(|a| a.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        Ok(Outcome::ok(
            json!({"action": "obs", "actions": actions}),
            format!("{count} OBS action(s):\n{human}"),
        ))
    }
}
