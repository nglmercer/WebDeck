//! Local administrators approve device capabilities. Only token hashes are
//! retained; tokens travel in Authorization or Socket.IO auth, never URLs.
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::adapters::config::atomic_replace;
use crate::domain::command::Capability;
use crate::domain::error::{AppError, ErrorCode};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub capabilities: Vec<Capability>,
    pub expires_at: u64,
    pub revoked: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    token_hash: String,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn hash(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn denied() -> AppError {
    AppError::new(
        ErrorCode::Unauthorized,
        "Device identity is missing, expired or revoked",
    )
}

pub struct SessionService {
    path: PathBuf,
    devices: Mutex<Vec<Device>>,
}

impl SessionService {
    pub fn open(path: PathBuf) -> Result<Self, AppError> {
        let devices = if path.exists() {
            serde_json::from_slice(&fs::read(&path).map_err(|_| denied())?).map_err(|_| denied())?
        } else {
            Vec::new()
        };
        Ok(Self {
            path,
            devices: Mutex::new(devices),
        })
    }

    fn persist(&self, devices: &[Device]) -> Result<(), AppError> {
        atomic_replace(
            &self.path,
            &serde_json::to_vec(devices).map_err(|_| denied())?,
        )
    }

    pub fn approve(
        &self,
        name: String,
        capabilities: Vec<Capability>,
        ttl: u64,
    ) -> Result<(Device, String), AppError> {
        if name.trim().is_empty()
            || name.len() > 128
            || ttl == 0
            || ttl > 86400
            || capabilities.contains(&Capability::Admin)
        {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Invalid device grant",
            ));
        }
        let mut random = [0u8; 32];
        getrandom::fill(&mut random).map_err(|_| {
            AppError::new(
                ErrorCode::ExecutionFailed,
                "Cannot generate device identity",
            )
        })?;
        let token: String = random.iter().map(|b| format!("{b:02x}")).collect();
        let token_hash = hash(&token);
        let device = Device {
            id: token_hash[..16].to_string(),
            name,
            capabilities,
            expires_at: now() + ttl,
            revoked: false,
            token_hash,
        };
        let mut devices = self.devices.lock().unwrap_or_else(|p| p.into_inner());
        if devices.len() >= 1024 {
            return Err(AppError::new(
                ErrorCode::CapacityExhausted,
                "Device capacity exhausted",
            ));
        }
        let mut next = devices.clone();
        next.push(device.clone());
        self.persist(&next)?;
        *devices = next;
        Ok((public(device), token))
    }

    pub fn authorize(&self, token: &str) -> Result<Vec<Capability>, AppError> {
        if token.len() != 64 {
            return Err(denied());
        }
        let digest = hash(token);
        self.devices
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .iter()
            .find(|d| {
                !d.revoked && d.expires_at > now() && constant_time_equal(&d.token_hash, &digest)
            })
            .map(|d| d.capabilities.clone())
            .ok_or_else(denied)
    }

    pub fn list(&self) -> Vec<Device> {
        self.devices
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .iter()
            .cloned()
            .map(public)
            .collect()
    }

    pub fn revoke(&self, id: &str) -> Result<(), AppError> {
        let mut devices = self.devices.lock().unwrap_or_else(|p| p.into_inner());
        let mut next = devices.clone();
        let device = next.iter_mut().find(|d| d.id == id).ok_or_else(denied)?;
        device.revoked = true;
        self.persist(&next)?;
        *devices = next;
        Ok(())
    }
}

fn public(mut device: Device) -> Device {
    device.token_hash.clear();
    device
}
fn constant_time_equal(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |different, (a, b)| different | (a ^ b))
        == 0
}

pub fn shared() -> Result<Arc<SessionService>, AppError> {
    static SERVICES: OnceLock<Mutex<std::collections::HashMap<PathBuf, Arc<SessionService>>>> =
        OnceLock::new();
    let path = crate::app::utils::settings::get_config::config_dir().join("devices.json");
    let mut services = SERVICES
        .get_or_init(|| Mutex::new(std::collections::HashMap::new()))
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    if let Some(service) = services.get(&path) {
        return Ok(service.clone());
    }
    let service = Arc::new(SessionService::open(path.clone())?);
    services.insert(path, service.clone());
    Ok(service)
}
