use crate::contracts::*;
use serde::Serialize;
use serde_json::Value;
use std::{collections::HashSet, sync::OnceLock};

#[derive(Debug, Clone, Serialize)]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
}
impl Error {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
    pub fn invalid() -> Self {
        Self::new(ErrorCode::InvalidInput, "Invalid v2 document")
    }
    pub fn execution() -> Self {
        Self::new(
            ErrorCode::ExecutionFailed,
            "The action could not be completed",
        )
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;
fn schema() -> &'static Value {
    static S: OnceLock<Value> = OnceLock::new();
    S.get_or_init(|| {
        serde_json::from_str(include_str!("../contracts/v2.schema.json")).expect("generated schema")
    })
}
pub fn validate(name: &str, value: &Value) -> Result<()> {
    check(&schema()["$defs"][name], value, 0)
}
fn check(s: &Value, v: &Value, depth: usize) -> Result<()> {
    if depth > 64 {
        return Err(Error::invalid());
    }
    if let Some(r) = s["$ref"].as_str() {
        return check(
            &schema()["$defs"][r.rsplit('/').next().unwrap_or("")],
            v,
            depth + 1,
        );
    }
    if let Some(options) = s["oneOf"].as_array() {
        return if options
            .iter()
            .filter(|s| check(s, v, depth + 1).is_ok())
            .count()
            == 1
        {
            Ok(())
        } else {
            Err(Error::invalid())
        };
    }
    if let Some(c) = s.get("const") {
        if c != v {
            return Err(Error::invalid());
        }
    }
    if let Some(e) = s["enum"].as_array() {
        if !e.contains(v) {
            return Err(Error::invalid());
        }
    }
    match s["type"].as_str() {
        Some("object") => {
            let o = v.as_object().ok_or_else(Error::invalid)?;
            if let Some(req) = s["required"].as_array() {
                for k in req {
                    if !o.contains_key(k.as_str().unwrap_or("")) {
                        return Err(Error::invalid());
                    }
                }
            }
            for (k, v) in o {
                if let Some(t) = s["properties"].get(k) {
                    check(t, v, depth + 1)?;
                } else if s["additionalProperties"] == false {
                    return Err(Error::invalid());
                } else if s["additionalProperties"].is_object() {
                    check(&s["additionalProperties"], v, depth + 1)?;
                }
            }
        }
        Some("array") => {
            let a = v.as_array().ok_or_else(Error::invalid)?;
            bound(s, a.len() as f64, "minItems", "maxItems")?;
            for v in a {
                check(&s["items"], v, depth + 1)?;
            }
        }
        Some("string") => {
            let t = v.as_str().ok_or_else(Error::invalid)?;
            bound(s, t.chars().count() as f64, "minLength", "maxLength")?;
            if t.contains('\0') {
                return Err(Error::invalid());
            }
            if s.get("pattern").is_some()
                && !t
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
            {
                return Err(Error::invalid());
            }
        }
        Some("integer") => {
            if !v.is_i64() && !v.is_u64() {
                return Err(Error::invalid());
            }
            bound(
                s,
                v.as_f64().ok_or_else(Error::invalid)?,
                "minimum",
                "maximum",
            )?;
        }
        Some("number") => bound(
            s,
            v.as_f64().ok_or_else(Error::invalid)?,
            "minimum",
            "maximum",
        )?,
        Some("null") => {
            if !v.is_null() {
                return Err(Error::invalid());
            }
        }
        Some("boolean") if !v.is_boolean() => return Err(Error::invalid()),
        _ => {}
    }
    Ok(())
}
fn bound(s: &Value, n: f64, min: &str, max: &str) -> Result<()> {
    if s[min].as_f64().is_some_and(|a| n < a) || s[max].as_f64().is_some_and(|b| n > b) {
        Err(Error::invalid())
    } else {
        Ok(())
    }
}
pub fn validate_command(c: &Command) -> Result<()> {
    validate(
        "Command",
        &serde_json::to_value(c).map_err(|_| Error::invalid())?,
    )?;
    if let Command::Fetch { url, .. } = c {
        let u = reqwest::Url::parse(url).map_err(|_| Error::invalid())?;
        if !matches!(u.scheme(), "http" | "https")
            || !u.username().is_empty()
            || u.password().is_some()
        {
            return Err(Error::invalid());
        }
    }
    Ok(())
}
pub fn validate_config(c: &Config) -> Result<()> {
    if c.schema_version != 2 {
        return Err(Error::new(
            ErrorCode::UnsupportedSchema,
            "Only canonical schema version 2 is supported; existing files were preserved",
        ));
    }
    validate(
        "Config",
        &serde_json::to_value(c).map_err(|_| Error::invalid())?,
    )?;
    if c.settings.automatic_updates {
        return Err(Error::new(
            ErrorCode::InvalidInput,
            "Automatic updates remain disabled until artifact verification is complete",
        ));
    }
    for n in &c.settings.allowed_networks {
        let (ip, prefix) = n
            .split_once('/')
            .map_or((n.as_str(), None), |(a, b)| (a, Some(b)));
        let ip: std::net::IpAddr = ip.parse().map_err(|_| Error::invalid())?;
        if let Some(p) = prefix {
            if p.parse::<u8>().map_err(|_| Error::invalid())? > if ip.is_ipv4() { 32 } else { 128 }
            {
                return Err(Error::invalid());
            }
        }
    }
    let mut ids = HashSet::new();
    let folders: HashSet<_> = c.layout.folders.iter().map(|f| f.id.as_str()).collect();
    for f in &c.layout.folders {
        if f.buttons.len() as u64 > c.layout.columns * c.layout.rows {
            return Err(Error::new(
                ErrorCode::InvalidInput,
                "The grid is too small for its buttons",
            ));
        }
        if !ids.insert(&f.id) {
            return Err(Error::invalid());
        }
        for b in &f.buttons {
            if !ids.insert(&b.id) {
                return Err(Error::invalid());
            }
            match &b.action {
                ButtonAction::Command { command } => {
                    validate_command(command)?;
                    validate_reference(c, command, &mut HashSet::new())?;
                }
                ButtonAction::Folder { folder_id } if !folders.contains(folder_id.as_str()) => {
                    return Err(Error::invalid())
                }
                _ => {}
            }
        }
    }
    Ok(())
}
pub fn decode_config(bytes: &[u8]) -> Result<Config> {
    let v: Value = serde_json::from_slice(bytes).map_err(|_| Error::invalid())?;
    if v["schema_version"] != 2 {
        return Err(Error::new(
            ErrorCode::UnsupportedSchema,
            "Only canonical schema version 2 is supported; no conversion was performed",
        ));
    }
    validate("Config", &v)?;
    let c: Config = serde_json::from_value(v).map_err(|_| Error::invalid())?;
    validate_config(&c)?;
    Ok(c)
}
pub fn id() -> String {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).expect("operating system entropy");
    b.iter().map(|b| format!("{b:02x}")).collect()
}
pub const ALL_CAPABILITIES: [Capability; 10] = [
    Capability::Read,
    Capability::Input,
    Capability::Audio,
    Capability::Window,
    Capability::Power,
    Capability::Script,
    Capability::Network,
    Capability::Plugin,
    Capability::Admin,
    Capability::Settings,
];

fn validate_reference(c: &Config, command: &Command, visited: &mut HashSet<String>) -> Result<()> {
    if let Command::Button { button_id } = command {
        if visited.len() > 8 || !visited.insert(button_id.clone()) {
            return Err(Error::new(
                ErrorCode::InvalidInput,
                "Invalid or cyclic button reference",
            ));
        }
        let command = c
            .layout
            .folders
            .iter()
            .flat_map(|f| &f.buttons)
            .find(|b| &b.id == button_id)
            .and_then(|b| {
                if let ButtonAction::Command { command } = &b.action {
                    Some(command)
                } else {
                    None
                }
            })
            .ok_or_else(Error::invalid)?;
        validate_reference(c, command, visited)?;
    }
    Ok(())
}
