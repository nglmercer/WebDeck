pub mod actions;
pub mod buttons;
pub mod client;
pub mod configuration;
pub mod doctor;
pub mod error;
pub mod folders;
pub mod integrations;
pub mod output;
pub mod plugins;
pub mod provisioning;

pub use client::{MutationReport, WebDeckAdminClient};
pub use error::{AdminError, ErrorKind, ExitCode, Result};
pub use output::{Emitter, Outcome, OutputMode};

use serde::Deserialize;
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct Connection {
    pub url: String,
    pub token: Option<String>,
    pub config_dir: PathBuf,
}

#[derive(Debug, Default, Deserialize)]
struct FileConnection {
    url: Option<String>,
    token: Option<String>,
    config_dir: Option<PathBuf>,
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
}

fn connection_file() -> Option<PathBuf> {
    home_dir().map(|home| home.join(".config").join("webdeck").join("config.toml"))
}

fn parse_url(url: &str) -> Result<String> {
    let parsed = reqwest::Url::parse(url)
        .map_err(|_| AdminError::invalid_arguments(format!("Invalid server URL: {url}")))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(AdminError::invalid_arguments(format!(
            "Server URL must be http or https: {url}"
        )));
    }
    Ok(url.trim_end_matches('/').to_string())
}

pub fn resolve_connection(
    url: Option<&str>,
    token: Option<&str>,
    config_dir: Option<PathBuf>,
) -> Result<Connection> {
    let file = connection_file()
        .filter(|p| p.exists())
        .map(|p| {
            let text = std::fs::read_to_string(&p).map_err(|_| {
                AdminError::invalid_arguments(format!(
                    "Cannot read connection file {}",
                    p.display()
                ))
            })?;
            toml::from_str::<FileConnection>(&text).map_err(|_| {
                AdminError::invalid_arguments(format!("Invalid connection file {}", p.display()))
            })
        })
        .transpose()?;
    let file = file.unwrap_or_default();
    let env_url = std::env::var("WEBDECK_URL").ok();
    let env_token = std::env::var("WEBDECK_ADMIN_TOKEN")
        .ok()
        .or_else(|| std::env::var("WEBDECK_DEVICE_TOKEN").ok());
    let raw = url
        .map(str::to_string)
        .or(env_url)
        .or(file.url)
        .unwrap_or_else(|| "http://127.0.0.1:5000".to_string());
    let token = token
        .map(str::to_string)
        .or(env_token)
        .or(file.token)
        .filter(|t| !t.trim().is_empty());
    let config_dir = config_dir
        .or_else(|| std::env::var_os("WEBDECK_CONFIG_DIR").map(PathBuf::from))
        .or(file.config_dir)
        .unwrap_or_else(|| PathBuf::from(".config"));
    Ok(Connection {
        url: parse_url(&raw)?,
        token,
        config_dir,
    })
}

pub fn connect(connection: &Connection, timeout: u64, verbose: u8) -> WebDeckAdminClient {
    WebDeckAdminClient::new(connection.url.clone())
        .with_token(connection.token.clone())
        .with_timeout(timeout)
        .with_verbose(verbose)
}

fn read_all<R: Read>(reader: R, limit: usize) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| AdminError::invalid_arguments(format!("Cannot read input: {e}")))?;
    if bytes.len() > limit {
        return Err(AdminError::invalid_arguments(format!(
            "Input exceeds {limit} bytes"
        )));
    }
    Ok(bytes)
}

pub fn read_bytes(file: Option<&str>) -> Result<Vec<u8>> {
    match file {
        Some(path) if path != "-" => std::fs::read(path)
            .map_err(|e| AdminError::invalid_arguments(format!("Cannot read {}: {e}", path))),
        _ => {
            use std::io::IsTerminal;
            if std::io::stdin().is_terminal() {
                return Err(AdminError::invalid_arguments(
                    "Provide a file with --file or pipe JSON on stdin",
                ));
            }
            read_all(&mut std::io::stdin(), MAX_INPUT_BYTES)
        }
    }
}

pub fn read_value(file: Option<&str>) -> Result<Value> {
    let bytes = read_bytes(file)?;
    serde_json::from_slice(&bytes)
        .map_err(|_| AdminError::invalid_arguments("Input is not valid JSON"))
}

pub fn read_typed<T: serde::de::DeserializeOwned>(file: Option<&str>) -> Result<T> {
    let value = read_value(file)?;
    serde_json::from_value(value)
        .map_err(|_| AdminError::invalid_arguments("Input does not match the expected shape"))
}

pub fn merge_patch(base: &Value, patch: &Value) -> Value {
    match (base, patch) {
        (Value::Object(base_map), Value::Object(patch_map)) => {
            let mut merged = base_map.clone();
            for (key, value) in patch_map {
                merged.insert(key.clone(), value.clone());
            }
            Value::Object(merged)
        }
        _ => patch.clone(),
    }
}

pub fn report_value(report: &MutationReport, extra: Value) -> Value {
    let mut value = serde_json::to_value(report).unwrap_or(Value::Null);
    if let (Value::Object(map), Value::Object(extra)) = (&mut value, extra) {
        map.extend(extra);
    }
    value
}

pub fn human_line(parts: &[(&str, String)]) -> String {
    parts
        .iter()
        .map(|(label, value)| format!("{label}: {value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(unix)]
pub fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| {
            AdminError::invalid_arguments(format!("Cannot write {}: {e}", path.display()))
        })?;
    std::io::Write::write_all(&mut file, bytes).map_err(|e| {
        AdminError::invalid_arguments(format!("Cannot write {}: {e}", path.display()))
    })?;
    Ok(())
}

#[cfg(not(unix))]
pub fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    std::fs::write(path, bytes)
        .map_err(|e| AdminError::invalid_arguments(format!("Cannot write {}: {e}", path.display())))
}
