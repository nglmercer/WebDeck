// Generated from contracts/v2.schema.json; run node tools/contracts/generate.mjs.
use crate::domain::command::Capability;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandRequest {
    pub message: String,
    pub request_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigRequest {
    pub revision: u64,
    pub config: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceRequest {
    pub name: String,
    pub capabilities: Vec<Capability>,
    pub ttl_seconds: u64,
}
