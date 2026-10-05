use super::error::{kind_from_status, AdminError, ErrorKind, Result};
use crate::contracts::*;
use crate::domain;
use serde::Serialize;
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct MutationReport {
    pub operation: String,
    pub revision: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_revision: Option<u64>,
    pub applied: bool,
    pub dry_run: bool,
}

impl MutationReport {
    pub fn unchanged(revision: u64) -> Self {
        Self {
            operation: "unchanged".into(),
            revision,
            previous_revision: None,
            applied: false,
            dry_run: false,
        }
    }
    pub fn applied(operation: &str, previous_revision: u64, revision: u64) -> Self {
        Self {
            operation: operation.into(),
            revision,
            previous_revision: Some(previous_revision),
            applied: true,
            dry_run: false,
        }
    }
    pub fn planned(operation: &str, revision: u64) -> Self {
        Self {
            operation: operation.into(),
            revision,
            previous_revision: None,
            applied: false,
            dry_run: true,
        }
    }
}

#[derive(Clone)]
pub struct WebDeckAdminClient {
    base_url: String,
    token: Option<String>,
    http: reqwest::Client,
    timeout: Duration,
    verbose: u8,
}

impl WebDeckAdminClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let base = base_url.into();
        Self {
            base_url: base.trim_end_matches('/').to_string(),
            token: None,
            http: reqwest::Client::new(),
            timeout: Duration::from_secs(35),
            verbose: 0,
        }
    }
    pub fn with_token(mut self, token: Option<String>) -> Self {
        self.token = token.filter(|t| !t.trim().is_empty());
        self
    }
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout = Duration::from_secs(seconds);
        self
    }
    pub fn with_verbose(mut self, verbose: u8) -> Self {
        self.verbose = verbose;
        self
    }
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
    fn url(&self, path: &str) -> String {
        format!("{}/api/v2/{}", self.base_url, path.trim_start_matches('/'))
    }
    fn transport_error(&self, url: &str, error: &reqwest::Error) -> AdminError {
        if error.is_connect() {
            AdminError::connection(format!("Cannot reach WebDeck at {url}"))
        } else if error.is_timeout() {
            AdminError::connection(format!("Request to {url} timed out"))
        } else {
            AdminError::connection(format!("Request to {url} failed: {error}"))
        }
    }
    async fn send(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<(u16, Vec<u8>)> {
        let url = self.url(path);
        if self.verbose > 0 {
            eprintln!("webdeckctl: -> {} {url}", method.as_str());
        }
        let mut request = self.http.request(method, &url).timeout(self.timeout);
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request
            .send()
            .await
            .map_err(|e| self.transport_error(&url, &e))?;
        let status = response.status().as_u16();
        if self.verbose > 0 {
            eprintln!("webdeckctl: <- {status}");
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|e| self.transport_error(&url, &e))?;
        Ok((status, bytes.to_vec()))
    }
    fn decode(bytes: &[u8]) -> Result<Value> {
        serde_json::from_slice(bytes).map_err(|_| {
            AdminError::new(ErrorKind::Generic, "WebDeck returned an invalid response")
        })
    }
    pub(crate) fn http_error(&self, status: u16, bytes: &[u8]) -> AdminError {
        if let Ok(value) = serde_json::from_slice::<Value>(bytes) {
            if let (Some(code), Some(message)) = (value["code"].as_str(), value["message"].as_str())
            {
                let mut error = AdminError::from_server_code(code, message, status);
                let mut details = value.get("details").cloned().unwrap_or_else(|| json!({}));
                details["status"] = json!(status);
                error.set_details(details);
                return error;
            }
            if value["state"].as_str() == Some("failed") {
                let code = value["code"].as_str().unwrap_or("");
                let message = value["message"].as_str().unwrap_or("Command failed");
                let mut error = AdminError::from_server_code(code, message, status);
                error.set_details(json!({"status": status}));
                return error;
            }
        }
        let text = String::from_utf8_lossy(bytes);
        let body: String = text.chars().take(300).collect();
        AdminError::with_details(
            kind_from_status(status),
            format!("WebDeck rejected the request with HTTP {status}"),
            json!({"status": status, "body": body}),
        )
    }
    async fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value> {
        let (status, bytes) = self.send(method, path, body.as_ref()).await?;
        if (200..300).contains(&status) {
            Self::decode(&bytes)
        } else {
            Err(self.http_error(status, &bytes))
        }
    }
    async fn get_typed<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let value = self.request(reqwest::Method::GET, path, None).await?;
        serde_json::from_value(value).map_err(|_| {
            AdminError::new(
                ErrorKind::Generic,
                "WebDeck returned an unexpected response shape",
            )
        })
    }
    pub(crate) async fn enrich_conflict(&self, error: AdminError, expected: u64) -> AdminError {
        if error.kind() != ErrorKind::RevisionConflict {
            return error;
        }
        let actual = self.get_config().await.ok().map(|c| c.revision);
        let mut error = error;
        let mut details = error.details().cloned().unwrap_or_else(|| json!({}));
        details["expected"] = json!(expected);
        details["actual"] = match actual {
            Some(revision) => json!(revision),
            None => Value::Null,
        };
        error.set_details(details);
        error
    }
    pub(crate) fn ensure_revision(&self, expected: Option<u64>, current: u64) -> Result<()> {
        match expected {
            Some(expected) if expected != current => Err(AdminError::with_details(
                ErrorKind::RevisionConflict,
                "Configuration revision changed",
                json!({"expected": expected, "actual": current}),
            )),
            _ => Ok(()),
        }
    }
    pub async fn version(&self) -> Result<ServerVersion> {
        self.get_typed("version").await
    }
    pub async fn boot(&self) -> Result<DeckBoot> {
        self.get_typed("boot").await
    }
    pub async fn get_config(&self) -> Result<ConfigResponse> {
        self.get_typed("config").await
    }
    pub async fn put_config(&self, revision: u64, config: Config) -> Result<ConfigResponse> {
        let response = self
            .request(
                reqwest::Method::PUT,
                "config",
                Some(json!({"revision": revision, "config": config})),
            )
            .await?;
        serde_json::from_value(response).map_err(|_| {
            AdminError::new(
                ErrorKind::Generic,
                "WebDeck returned an unexpected response shape",
            )
        })
    }
    pub async fn put_settings(&self, revision: u64, settings: Settings) -> Result<ConfigResponse> {
        let response = self
            .request(
                reqwest::Method::PUT,
                "settings",
                Some(json!({"revision": revision, "settings": settings})),
            )
            .await?;
        serde_json::from_value(response).map_err(|_| {
            AdminError::new(
                ErrorKind::Generic,
                "WebDeck returned an unexpected response shape",
            )
        })
    }
    pub async fn create_folder(&self, revision: u64, folder: Folder) -> Result<ConfigResponse> {
        let response = self
            .request(
                reqwest::Method::POST,
                "folders",
                Some(json!({"revision": revision, "folder": folder})),
            )
            .await?;
        serde_json::from_value(response).map_err(|_| {
            AdminError::new(
                ErrorKind::Generic,
                "WebDeck returned an unexpected response shape",
            )
        })
    }
    pub async fn delete_folder(&self, revision: u64, id: &str) -> Result<ConfigResponse> {
        let response = self
            .request(
                reqwest::Method::DELETE,
                &format!("folders/{}", encode(id)),
                Some(json!({"revision": revision})),
            )
            .await?;
        serde_json::from_value(response).map_err(|_| {
            AdminError::new(
                ErrorKind::Generic,
                "WebDeck returned an unexpected response shape",
            )
        })
    }
    pub async fn create_button(
        &self,
        folder: &str,
        revision: u64,
        button: Button,
    ) -> Result<ConfigResponse> {
        let response = self
            .request(
                reqwest::Method::POST,
                &format!("folders/{}/buttons", encode(folder)),
                Some(json!({"revision": revision, "button": button})),
            )
            .await?;
        serde_json::from_value(response).map_err(|_| {
            AdminError::new(
                ErrorKind::Generic,
                "WebDeck returned an unexpected response shape",
            )
        })
    }
    pub async fn update_button(
        &self,
        folder: &str,
        revision: u64,
        button: Button,
    ) -> Result<ConfigResponse> {
        let response = self
            .request(
                reqwest::Method::PUT,
                &format!("folders/{}/buttons/{}", encode(folder), encode(&button.id)),
                Some(json!({"revision": revision, "button": button})),
            )
            .await?;
        serde_json::from_value(response).map_err(|_| {
            AdminError::new(
                ErrorKind::Generic,
                "WebDeck returned an unexpected response shape",
            )
        })
    }
    pub async fn delete_button(
        &self,
        folder: &str,
        id: &str,
        revision: u64,
    ) -> Result<ConfigResponse> {
        let response = self
            .request(
                reqwest::Method::DELETE,
                &format!("folders/{}/buttons/{}", encode(folder), encode(id)),
                Some(json!({"revision": revision})),
            )
            .await?;
        serde_json::from_value(response).map_err(|_| {
            AdminError::new(
                ErrorKind::Generic,
                "WebDeck returned an unexpected response shape",
            )
        })
    }
    pub async fn catalog(&self) -> Result<CatalogResponse> {
        self.get_typed("commands").await
    }
    pub async fn runtime_status(&self) -> Result<RuntimeSnapshot> {
        self.get_typed("runtime").await
    }
    pub async fn runtime_reload(&self) -> Result<RuntimeSnapshot> {
        let response = self
            .request(reqwest::Method::POST, "runtime/reload", Some(json!({})))
            .await?;
        serde_json::from_value(response).map_err(|_| {
            AdminError::new(
                ErrorKind::Generic,
                "WebDeck returned an unexpected response shape",
            )
        })
    }
    pub async fn set_plugin_enabled(&self, id: &str, enabled: bool) -> Result<RuntimeSnapshot> {
        let response = self
            .request(
                reqwest::Method::PUT,
                &format!("runtime/plugins/{}", encode(id)),
                Some(json!({"enabled": enabled})),
            )
            .await?;
        serde_json::from_value(response).map_err(|_| {
            AdminError::new(
                ErrorKind::Generic,
                "WebDeck returned an unexpected response shape",
            )
        })
    }
    pub async fn integration_status(&self) -> Result<IntegrationStatus> {
        self.get_typed("integrations/status").await
    }
    pub async fn obs_check(&self) -> Result<IntegrationStatus> {
        let response = self
            .request(
                reqwest::Method::POST,
                "integrations/obs/check",
                Some(json!({})),
            )
            .await?;
        serde_json::from_value(response).map_err(|_| {
            AdminError::new(
                ErrorKind::Generic,
                "WebDeck returned an unexpected response shape",
            )
        })
    }
    pub async fn run_command(&self, command: Command) -> Result<Value> {
        domain::validate_command(&command).map_err(AdminError::from_domain)?;
        let request_id = domain::id()
            .map_err(|_| AdminError::new(ErrorKind::Generic, "Cannot generate a request id"))?;
        let body = json!({"request_id": request_id, "command": command});
        let url = self.url("commands");
        let mut request = self.http.post(&url).timeout(self.timeout).json(&body);
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        if self.verbose > 0 {
            eprintln!("webdeckctl: -> POST {url}");
        }
        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                return if error.is_connect() {
                    Err(self.transport_error(&url, &error))
                } else {
                    Err(AdminError::with_details(
                        ErrorKind::Connection,
                        "Execution outcome is unknown. No retry was sent.",
                        json!({"url": url}),
                    ))
                };
            }
        };
        let status = response.status().as_u16();
        if self.verbose > 0 {
            eprintln!("webdeckctl: <- {status}");
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|_| {
                AdminError::with_details(
                    ErrorKind::Connection,
                    "Execution outcome is unknown. No retry was sent.",
                    json!({"url": url}),
                )
            })?
            .to_vec();
        let value = Self::decode(&bytes)?;
        match value["state"].as_str() {
            Some("completed") => Ok(value["result"].clone()),
            Some("failed") => {
                let code = value["code"].as_str().unwrap_or("");
                let message = value["message"].as_str().unwrap_or("Command failed");
                let mut error = AdminError::from_server_code(code, message, status);
                if error.kind() == ErrorKind::Generic && matches!(command, Command::Obs { .. }) {
                    error = AdminError::new(ErrorKind::Integration, message);
                }
                let command_name = serde_json::to_value(&command)
                    .ok()
                    .and_then(|value| value.get("type").cloned())
                    .unwrap_or(Value::Null);
                error.set_details(json!({"status": status, "command": command_name}));
                Err(error)
            }
            _ => Err(self.http_error(status, &bytes)),
        }
    }
}

fn encode(segment: &str) -> String {
    let mut encoded = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}
