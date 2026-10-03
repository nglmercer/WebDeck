use crate::{
    contracts::*,
    domain::{self, Error, Result},
    storage,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn hash(t: &str) -> String {
    Sha256::digest(t.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn denied() -> Error {
    Error::new(
        ErrorCode::Unauthorized,
        "Pairing required, or device expired or revoked",
    )
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    device: Device,
    token_hash: String,
}
pub struct Sessions {
    path: PathBuf,
    devices: Mutex<Vec<Stored>>,
}
impl Sessions {
    pub fn open(path: PathBuf) -> Result<Self> {
        let d = if path.exists() {
            serde_json::from_slice(&fs::read(&path).map_err(|_| denied())?).map_err(|_| denied())?
        } else {
            Vec::new()
        };
        Ok(Self {
            path,
            devices: Mutex::new(d),
        })
    }
    fn read(&self) -> Result<Vec<Stored>> {
        if self.path.exists() {
            let b = fs::read(&self.path).map_err(|_| denied())?;
            if b.len() > 1024 * 1024 {
                return Err(denied());
            }
            serde_json::from_slice(&b).map_err(|_| denied())
        } else {
            Ok(Vec::new())
        }
    }
    pub fn authorize(&self, token: Option<&str>, local: bool) -> Result<Vec<Capability>> {
        if let Some(t) = token {
            let h = hash(t);
            let d = self.read()?;
            return d
                .iter()
                .find(|s| s.token_hash == h && !s.device.revoked && s.device.expires_at > now())
                .map(|s| s.device.capabilities.clone())
                .ok_or_else(denied);
        }
        if local {
            Ok(domain::ALL_CAPABILITIES.to_vec())
        } else {
            Err(denied())
        }
    }
    pub fn list(&self) -> Vec<Device> {
        self.devices
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .iter()
            .map(|s| s.device.clone())
            .collect()
    }
    pub fn approve(&self, r: DeviceRequest) -> Result<DeviceApproval> {
        domain::validate(
            "DeviceRequest",
            &serde_json::to_value(&r).map_err(|_| Error::invalid())?,
        )?;
        if r.capabilities.contains(&Capability::Admin) {
            return Err(Error::new(
                ErrorCode::Forbidden,
                "Administrative actions are local only",
            ));
        }
        let token = format!("{}{}", domain::id()?, domain::id()?);
        let device = Device {
            id: domain::id()?,
            name: r.name,
            capabilities: r.capabilities,
            expires_at: now() + r.ttl_seconds,
            revoked: false,
        };
        let mut d = self.devices.lock().unwrap_or_else(|p| p.into_inner());
        let _l = storage::lock(&self.path)?;
        let mut next = self.read()?;
        if next.len() >= 512 {
            return Err(Error::new(
                ErrorCode::CapacityExhausted,
                "Device capacity exhausted",
            ));
        }
        next.push(Stored {
            device: device.clone(),
            token_hash: hash(&token),
        });
        storage::atomic_replace(
            &self.path,
            &serde_json::to_vec(&next).map_err(|_| Error::invalid())?,
        )?;
        *d = next;
        Ok(DeviceApproval {
            api_version: 2,
            device,
            token,
        })
    }
    pub fn revoke(&self, id: &str) -> Result<()> {
        let mut d = self.devices.lock().unwrap_or_else(|p| p.into_inner());
        let _l = storage::lock(&self.path)?;
        let mut next = self.read()?;
        let s = next
            .iter_mut()
            .find(|s| s.device.id == id)
            .ok_or_else(Error::invalid)?;
        s.device.revoked = true;
        storage::atomic_replace(
            &self.path,
            &serde_json::to_vec(&next).map_err(|_| Error::invalid())?,
        )?;
        *d = next;
        Ok(())
    }
}
