use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::{AppError, ErrorCode};

pub const SCHEMA_VERSION: u64 = 2;

/// Extension data and insertion order remain in the JSON document. Typed
/// accessors validate known fields without discarding plugin/custom data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(transparent)]
pub struct ConfigDocument(pub Value);

impl ConfigDocument {
    pub fn validate(value: Value) -> Result<Self, AppError> {
        let invalid = || AppError::new(ErrorCode::InvalidInput, "Invalid configuration");
        let root = value.as_object().ok_or_else(invalid)?;
        for key in ["settings", "front", "url"] {
            if root.get(key).is_some_and(|v| !v.is_object()) {
                return Err(invalid());
            }
        }
        if root.get("schema_version").and_then(Value::as_u64) != Some(SCHEMA_VERSION) {
            return Err(AppError::new(ErrorCode::UnsupportedSchema,
                "Only schema_version 2 is supported. Preserve the data directory and use the deprecated v1 branch for older configurations."));
        }
        if let Some(buttons) = value.pointer("/front/buttons") {
            let folders = buttons.as_object().ok_or_else(invalid)?;
            if folders.values().any(|v| !v.is_array()) {
                return Err(invalid());
            }
        }
        if let Some(policy) = value.pointer("/settings/v2_security") {
            if !matches!(policy.as_str(), Some("paired")) {
                return Err(invalid());
            }
        }
        for path in ["/front/themes", "/front/background"] {
            if let Some(list) = value.pointer(path) {
                if !list
                    .as_array()
                    .is_some_and(|items| items.iter().all(Value::is_string))
                {
                    return Err(invalid());
                }
            }
        }
        if let Some(networks) = value.pointer("/settings/allowed_networks") {
            let networks = networks.as_array().ok_or_else(invalid)?;
            for network in networks {
                let text = network.as_str().ok_or_else(invalid)?;
                let (ip, prefix) = text
                    .split_once('/')
                    .map_or((text, None), |(ip, prefix)| (ip, Some(prefix)));
                let address = ip.parse::<std::net::IpAddr>().map_err(|_| invalid())?;
                if let Some(prefix) = prefix {
                    let prefix: u8 = prefix.parse().map_err(|_| invalid())?;
                    if prefix > if address.is_ipv4() { 32 } else { 128 } {
                        return Err(invalid());
                    }
                }
            }
        }
        for key in ["height", "width"] {
            if let Some(dim) = value.pointer(&format!("/front/{key}")) {
                let number = dim.as_u64();
                if !matches!(number, Some(1..=128)) {
                    return Err(invalid());
                }
            }
        }
        Ok(Self(value))
    }
}
