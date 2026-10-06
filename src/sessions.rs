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
struct Pending {
    request: PairingPending,
    secret_hash: String,
    state: String,
    token: String,
}
pub struct Sessions {
    path: PathBuf,
    devices: Mutex<Vec<Stored>>,
    pending: Mutex<Vec<Pending>>,
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
            pending: Mutex::new(Vec::new()),
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

impl Sessions {
    pub fn request_pairing(
        &self,
        name: String,
        address: std::net::IpAddr,
    ) -> Result<PairingChallenge> {
        domain::validate("PairingRequest", &serde_json::json!({"name":name}))?;
        if name.trim().is_empty() {
            return Err(Error::invalid());
        }
        let mut pending = self.pending.lock().unwrap_or_else(|p| p.into_inner());
        pending.retain(|p| p.request.expires_at > now());
        if pending.len() >= 32
            || pending
                .iter()
                .filter(|p| p.request.address == address.to_string())
                .count()
                >= 2
        {
            return Err(Error::new(
                ErrorCode::CapacityExhausted,
                "Too many pairing requests. Wait two minutes and try again.",
            ));
        }
        let id = domain::id()?;
        let secret = format!("{}{}", domain::id()?, domain::id()?);
        let code = format!(
            "{:06}",
            u32::from_str_radix(&domain::id()?[..6], 16).map_err(|_| Error::execution())?
                % 1_000_000
        );
        let expires_at = now() + 120;
        pending.push(Pending {
            request: PairingPending {
                id: id.clone(),
                name: name.trim().into(),
                address: address.to_string(),
                code: code.clone(),
                expires_at,
            },
            secret_hash: hash(&secret),
            state: "pending".into(),
            token: String::new(),
        });
        Ok(PairingChallenge {
            api_version: 2,
            id,
            secret,
            code,
            expires_at,
        })
    }
    pub fn pending_pairings(&self) -> PairingList {
        let mut pending = self.pending.lock().unwrap_or_else(|p| p.into_inner());
        pending.retain(|p| p.request.expires_at > now());
        PairingList {
            api_version: 2,
            requests: pending
                .iter()
                .filter(|p| p.state == "pending")
                .map(|p| p.request.clone())
                .collect(),
        }
    }
    pub fn claim_pairing(
        &self,
        claim: PairingClaim,
        address: std::net::IpAddr,
    ) -> Result<PairingResult> {
        let pending = self.pending.lock().unwrap_or_else(|p| p.into_inner());
        let Some(p) = pending.iter().find(|p| {
            p.request.id == claim.id
                && p.secret_hash == hash(&claim.secret)
                && p.request.address == address.to_string()
        }) else {
            return Err(denied());
        };
        if p.request.expires_at <= now() {
            return Ok(PairingResult {
                api_version: 2,
                state: "expired".into(),
                token: String::new(),
            });
        }
        Ok(PairingResult {
            api_version: 2,
            state: p.state.clone(),
            token: p.token.clone(),
        })
    }
    pub fn accept_pairing(&self, id: &str, mut grant: DeviceRequest) -> Result<()> {
        let mut pending = self.pending.lock().unwrap_or_else(|p| p.into_inner());
        let p = pending
            .iter_mut()
            .find(|p| p.request.id == id && p.state == "pending" && p.request.expires_at > now())
            .ok_or_else(Error::invalid)?;
        grant.name = p.request.name.clone();
        let approval = self.approve(grant)?;
        p.token = approval.token;
        p.state = "approved".into();
        Ok(())
    }
    pub fn reject_pairing(&self, id: &str) -> Result<()> {
        let mut pending = self.pending.lock().unwrap_or_else(|p| p.into_inner());
        let p = pending
            .iter_mut()
            .find(|p| p.request.id == id && p.state == "pending")
            .ok_or_else(Error::invalid)?;
        p.state = "rejected".into();
        Ok(())
    }
}
