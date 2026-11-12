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
fn expiry(ttl: u64) -> u64 {
    if ttl == 315360000 {
        9007199254740991
    } else {
        now() + ttl
    }
}
fn hash(t: &str) -> String {
    Sha256::digest(t.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn active(device: &Device) -> Result<()> {
    if device.revoked {
        return Err(Error::new(
            ErrorCode::DeviceRevoked,
            "This device's access was revoked. Request approval again.",
        ));
    }
    if device.expires_at <= now() {
        return Err(Error::new(
            ErrorCode::AuthorizationExpired,
            "This device's access expired. Request approval again.",
        ));
    }
    Ok(())
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
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct History {
    schema_version: u64,
    last_seen: std::collections::HashMap<String, u64>,
}
struct PermissionUpgrade {
    request: PermissionPending,
    state: String,
}
struct Invitation {
    hash: String,
    expires_at: u64,
    request_id: Option<String>,
}
#[derive(Default)]
struct Presence {
    active: usize,
    last_seen: u64,
}
struct Pending {
    request: PairingPending,
    secret_hash: String,
    state: String,
    token: String,
}
#[derive(Clone)]
pub struct ConnectionEvent {
    pub name: &'static str,
    pub device_id: Option<String>,
    pub value: serde_json::Value,
}
pub struct Sessions {
    path: PathBuf,
    events: tokio::sync::broadcast::Sender<ConnectionEvent>,
    attempts: Mutex<std::collections::HashMap<String, (u64, u32)>>,
    devices: Mutex<Vec<Stored>>,
    pending: Mutex<Vec<Pending>>,
    invitations: Mutex<Vec<Invitation>>,
    upgrades: Mutex<Vec<PermissionUpgrade>>,
    presence: Mutex<std::collections::HashMap<String, Presence>>,
}
impl Sessions {
    pub fn open(path: PathBuf) -> Result<Self> {
        let d = if path.exists() {
            serde_json::from_slice(&fs::read(&path).map_err(|_| denied())?).map_err(|_| denied())?
        } else {
            Vec::new()
        };
        let history_path = path.with_file_name("connection-history.v2.json");
        let presence = match fs::read(history_path) {
            Ok(bytes) if bytes.len() <= 1024 * 1024 => {
                let history: History = serde_json::from_slice(&bytes).map_err(|_| {
                    Error::new(
                        ErrorCode::PersistenceFailed,
                        "Cannot read connection history",
                    )
                })?;
                if history.schema_version != 1 {
                    return Err(Error::new(
                        ErrorCode::PersistenceFailed,
                        "Unsupported connection history version",
                    ));
                }
                history
                    .last_seen
                    .into_iter()
                    .map(|(id, last_seen)| {
                        (
                            id,
                            Presence {
                                active: 0,
                                last_seen,
                            },
                        )
                    })
                    .collect()
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Default::default(),
            _ => {
                return Err(Error::new(
                    ErrorCode::PersistenceFailed,
                    "Cannot read connection history",
                ))
            }
        };
        Ok(Self {
            path,
            events: tokio::sync::broadcast::channel(128).0,
            attempts: Mutex::new(Default::default()),
            devices: Mutex::new(d),
            pending: Mutex::new(Vec::new()),
            invitations: Mutex::new(Vec::new()),
            upgrades: Mutex::new(Vec::new()),
            presence: Mutex::new(presence),
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
            let stored = d.iter().find(|s| s.token_hash == h).ok_or_else(denied)?;
            active(&stored.device)?;
            return Ok(stored.device.capabilities.clone());
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
            expires_at: expiry(r.ttl_seconds),
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
        self.changed(
            "device.revoked",
            Some(id.into()),
            serde_json::json!({"id":id}),
        );
        Ok(())
    }
}

impl Sessions {
    pub fn request_pairing(
        &self,
        name: String,
        address: std::net::IpAddr,
    ) -> Result<PairingChallenge> {
        self.limit(&format!("request:{address}"), 10)?;
        domain::validate("PairingRequest", &serde_json::json!({"name":name}))?;
        if name.trim().is_empty() {
            return Err(Error::invalid());
        }
        let mut pending = self.pending.lock().unwrap_or_else(|p| p.into_inner());
        pending.retain(|p| p.request.expires_at > now());
        if pending.len() >= 32
            || pending
                .iter()
                .filter(|p| {
                    p.request.address == address.to_string()
                        && (p.state == "pending" || (p.state == "approved" && !p.token.is_empty()))
                })
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
        self.changed(
            "pairing.requested",
            None,
            serde_json::json!({"api_version":2}),
        );
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
        self.limit(&format!("claim:{address}"), 180)?;
        let mut pending = self.pending.lock().unwrap_or_else(|p| p.into_inner());
        let Some(p) = pending.iter_mut().find(|p| {
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
        let state = if p.state == "approved" && p.token.is_empty() {
            "expired".into()
        } else {
            p.state.clone()
        };
        let token = std::mem::take(&mut p.token);
        Ok(PairingResult {
            api_version: 2,
            state,
            token,
        })
    }
    pub fn accept_pairing(&self, id: &str, grant: DeviceRequest) -> Result<()> {
        let mut pending = self.pending.lock().unwrap_or_else(|p| p.into_inner());
        let p = pending
            .iter_mut()
            .find(|p| p.request.id == id && p.state == "pending" && p.request.expires_at > now())
            .ok_or_else(Error::invalid)?;
        let approval = self.approve(grant)?;
        p.token = approval.token;
        p.state = "approved".into();
        self.changed("pairing.approved", None, serde_json::json!({"id":id}));
        Ok(())
    }
    pub fn reject_pairing(&self, id: &str) -> Result<()> {
        let mut pending = self.pending.lock().unwrap_or_else(|p| p.into_inner());
        let p = pending
            .iter_mut()
            .find(|p| p.request.id == id && p.state == "pending")
            .ok_or_else(Error::invalid)?;
        p.state = "rejected".into();
        self.changed("pairing.rejected", None, serde_json::json!({"id":id}));
        Ok(())
    }
}

impl Sessions {
    pub fn invitation(&self) -> Result<(String, u64)> {
        let mut invitations = self.invitations.lock().unwrap_or_else(|p| p.into_inner());
        invitations.retain(|i| i.expires_at > now());
        if invitations.len() >= 32 {
            return Err(Error::new(
                ErrorCode::CapacityExhausted,
                "Too many connection invitations",
            ));
        }
        let secret = format!("{}{}", domain::id()?, domain::id()?);
        let expires_at = now() + 120;
        invitations.push(Invitation {
            hash: hash(&secret),
            expires_at,
            request_id: None,
        });
        Ok((secret, expires_at))
    }
    pub fn validate_invitation(&self, invitation: &str) -> Result<()> {
        if self
            .invitations
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .iter()
            .any(|i| i.hash == hash(invitation) && i.expires_at > now() && i.request_id.is_none())
        {
            Ok(())
        } else {
            Err(denied())
        }
    }
    pub fn invited_pairing(
        &self,
        invitation: &str,
        name: String,
        address: std::net::IpAddr,
    ) -> Result<PairingChallenge> {
        self.limit(&format!("invitation:{address}"), 10)?;
        let mut invitations = self.invitations.lock().unwrap_or_else(|p| p.into_inner());
        let index = invitations
            .iter()
            .position(|i| {
                i.hash == hash(invitation) && i.expires_at > now() && i.request_id.is_none()
            })
            .ok_or_else(denied)?;
        let challenge = self.request_pairing(name, address)?;
        invitations[index].request_id = Some(challenge.id.clone());
        Ok(challenge)
    }
    pub fn identify(&self, token: &str) -> Result<Device> {
        let h = hash(token);
        let stored = self
            .read()?
            .into_iter()
            .find(|s| s.token_hash == h)
            .ok_or_else(denied)?;
        active(&stored.device)?;
        Ok(stored.device)
    }
    pub fn update(&self, id: &str, update: DeviceUpdate) -> Result<Device> {
        domain::validate(
            "DeviceUpdate",
            &serde_json::to_value(&update).map_err(|_| Error::invalid())?,
        )?;
        if update.capabilities.contains(&Capability::Admin) {
            return Err(Error::new(
                ErrorCode::Forbidden,
                "Administrative actions are local only",
            ));
        }
        let mut devices = self.devices.lock().unwrap_or_else(|p| p.into_inner());
        let _lock = storage::lock(&self.path)?;
        let mut next = self.read()?;
        let stored = next
            .iter_mut()
            .find(|s| s.device.id == id && !s.device.revoked)
            .ok_or_else(Error::invalid)?;
        stored.device.name = update.name;
        stored.device.capabilities = update.capabilities;
        stored.device.expires_at = expiry(update.ttl_seconds);
        let device = stored.device.clone();
        storage::atomic_replace(
            &self.path,
            &serde_json::to_vec(&next).map_err(|_| Error::invalid())?,
        )?;
        *devices = next;
        self.changed(
            "device.permissions_changed",
            Some(id.into()),
            serde_json::to_value(&device).map_err(|_| Error::invalid())?,
        );
        Ok(device)
    }
    pub fn connected(&self, token: &str) -> Result<String> {
        let device = self.identify(token)?;
        let mut presence = self.presence.lock().unwrap_or_else(|p| p.into_inner());
        let state = presence.entry(device.id.clone()).or_default();
        state.active += 1;
        state.last_seen = now();
        self.changed(
            "device.connected",
            Some(device.id.clone()),
            serde_json::json!({"id": device.id}),
        );
        drop(presence);
        if let Err(error) = self.persist_history() {
            self.disconnected(&device.id);
            return Err(error);
        }
        Ok(device.id)
    }
    pub fn disconnected(&self, id: &str) {
        if let Some(state) = self
            .presence
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get_mut(id)
        {
            state.active = state.active.saturating_sub(1);
            state.last_seen = now();
            if state.active == 0 {
                self.changed(
                    "device.disconnected",
                    Some(id.into()),
                    serde_json::json!({"id":id}),
                );
            }
        }
    }
    pub fn device_presence(&self) -> DevicePresenceList {
        let devices = self.list();
        let presence = self.presence.lock().unwrap_or_else(|p| p.into_inner());
        DevicePresenceList {
            api_version: 2,
            devices: devices
                .into_iter()
                .map(|device| {
                    let state = presence.get(&device.id);
                    let online = !device.revoked
                        && device.expires_at > now()
                        && state.is_some_and(|s| s.active > 0);
                    let last_seen = state.map_or(0, |s| s.last_seen);
                    DevicePresence {
                        api_version: 2,
                        device,
                        online,
                        last_seen,
                    }
                })
                .collect(),
        }
    }
}

impl Sessions {
    pub fn request_permissions(
        &self,
        token: &str,
        capabilities: Vec<Capability>,
    ) -> Result<PermissionPending> {
        let device = self.identify(token)?;
        if capabilities
            .iter()
            .any(|c| !crate::connection::permissions::REMOTE.contains(c))
        {
            return Err(Error::new(
                ErrorCode::Forbidden,
                "Permission cannot be granted remotely",
            ));
        }
        let mut upgrades = self.upgrades.lock().unwrap_or_else(|p| p.into_inner());
        upgrades.retain(|u| u.request.expires_at > now());
        if let Some(existing) = upgrades
            .iter()
            .find(|u| u.request.device.id == device.id && u.state == "pending")
        {
            return Ok(existing.request.clone());
        }
        if upgrades.len() >= 32 {
            return Err(Error::new(
                ErrorCode::CapacityExhausted,
                "Too many permission requests",
            ));
        }
        let request = PermissionPending {
            id: domain::id()?,
            device,
            capabilities,
            expires_at: now() + 120,
        };
        self.changed(
            "device.permission_requested",
            None,
            serde_json::json!({"api_version":2}),
        );
        upgrades.push(PermissionUpgrade {
            request: request.clone(),
            state: "pending".into(),
        });
        Ok(request)
    }
    pub fn permission_requests(&self) -> PermissionRequests {
        let mut upgrades = self.upgrades.lock().unwrap_or_else(|p| p.into_inner());
        upgrades.retain(|u| u.request.expires_at > now());
        PermissionRequests {
            api_version: 2,
            requests: upgrades
                .iter()
                .filter(|u| u.state == "pending")
                .map(|u| u.request.clone())
                .collect(),
        }
    }
    pub fn decide_permissions(&self, id: &str, approve: bool) -> Result<()> {
        let mut upgrades = self.upgrades.lock().unwrap_or_else(|p| p.into_inner());
        let index = upgrades
            .iter()
            .position(|u| {
                u.request.id == id && u.request.expires_at > now() && u.state == "pending"
            })
            .ok_or_else(Error::invalid)?;
        if approve {
            let request = &upgrades[index].request;
            let mut devices = self.devices.lock().unwrap_or_else(|p| p.into_inner());
            let _lock = storage::lock(&self.path)?;
            let mut next = self.read()?;
            let stored = next
                .iter_mut()
                .find(|s| {
                    s.device.id == request.device.id
                        && !s.device.revoked
                        && s.device.expires_at > now()
                })
                .ok_or_else(denied)?;
            for capability in &request.capabilities {
                if !stored.device.capabilities.contains(capability) {
                    stored.device.capabilities.push(*capability);
                }
            }
            storage::atomic_replace(
                &self.path,
                &serde_json::to_vec(&next).map_err(|_| Error::invalid())?,
            )?;
            *devices = next;
            self.changed(
                "device.permissions_changed",
                Some(request.device.id.clone()),
                serde_json::json!({"id":request.device.id}),
            );
        }
        upgrades[index].state = if approve {
            "approved".into()
        } else {
            "rejected".into()
        };
        Ok(())
    }
}

impl Sessions {
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<ConnectionEvent> {
        self.events.subscribe()
    }
    fn changed(&self, name: &'static str, device_id: Option<String>, value: serde_json::Value) {
        let _ = self.events.send(ConnectionEvent {
            name,
            device_id,
            value,
        });
    }
    fn limit(&self, key: &str, ceiling: u32) -> Result<()> {
        let mut attempts = self.attempts.lock().unwrap_or_else(|p| p.into_inner());
        attempts.retain(|_, (expires, _)| *expires > now());
        if attempts.len() >= 1024 && !attempts.contains_key(key) {
            return Err(Error::new(
                ErrorCode::CapacityExhausted,
                "Pairing rate limit reached",
            ));
        }
        let entry = attempts.entry(key.into()).or_insert((now() + 60, 0));
        if entry.1 >= ceiling {
            return Err(Error::new(
                ErrorCode::CapacityExhausted,
                "Pairing rate limit reached. Wait a minute and retry.",
            ));
        }
        entry.1 += 1;
        Ok(())
    }
}

impl Sessions {
    pub fn persist_history(&self) -> Result<()> {
        let last_seen = self
            .presence
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .iter()
            .map(|(id, state)| (id.clone(), state.last_seen))
            .collect();
        let path = self.path.with_file_name("connection-history.v2.json");
        let _lock = storage::lock(&path)?;
        storage::atomic_replace(
            &path,
            &serde_json::to_vec(&History {
                schema_version: 1,
                last_seen,
            })
            .map_err(|_| Error::invalid())?,
        )
    }
}

impl Sessions {
    pub fn renew(&self, id: &str, ttl_seconds: u64) -> Result<Device> {
        domain::validate(
            "DeviceRenew",
            &serde_json::json!({"ttl_seconds":ttl_seconds}),
        )?;
        let mut devices = self.devices.lock().unwrap_or_else(|p| p.into_inner());
        let _lock = storage::lock(&self.path)?;
        let mut next = self.read()?;
        let stored = next
            .iter_mut()
            .find(|s| s.device.id == id && !s.device.revoked)
            .ok_or_else(denied)?;
        stored.device.expires_at = expiry(ttl_seconds);
        let device = stored.device.clone();
        storage::atomic_replace(
            &self.path,
            &serde_json::to_vec(&next).map_err(|_| Error::invalid())?,
        )?;
        *devices = next;
        self.changed(
            "device.permissions_changed",
            Some(id.into()),
            serde_json::json!({"id":id}),
        );
        Ok(device)
    }
}

impl Sessions {
    pub fn invitation_progress(&self, invitation: &str) -> ConnectionProgress {
        let invitations = self.invitations.lock().unwrap_or_else(|p| p.into_inner());
        let state = match invitations
            .iter()
            .find(|i| i.hash == hash(invitation) && i.expires_at > now())
        {
            Some(invitation) => match &invitation.request_id {
                Some(id) => self
                    .pending
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .iter()
                    .find(|p| &p.request.id == id)
                    .map_or("expired".into(), |p| p.state.clone()),
                None => "waiting".into(),
            },
            None => "expired".into(),
        };
        ConnectionProgress {
            api_version: 2,
            state,
        }
    }
    pub fn cancel_invitation(&self, invitation: &str) -> Result<()> {
        let mut invitations = self.invitations.lock().unwrap_or_else(|p| p.into_inner());
        let Some(index) = invitations.iter().position(|i| i.hash == hash(invitation)) else {
            return Ok(());
        };
        if let Some(id) = &invitations[index].request_id {
            let mut pending = self.pending.lock().unwrap_or_else(|p| p.into_inner());
            if let Some(request) = pending
                .iter_mut()
                .find(|p| &p.request.id == id && p.state == "pending")
            {
                request.state = "rejected".into();
            }
        }
        invitations.remove(index);
        Ok(())
    }
}

impl Sessions {
    pub fn permission_progress(&self, token: &str, id: &str) -> Result<ConnectionProgress> {
        let device = self.identify(token)?;
        let upgrades = self.upgrades.lock().unwrap_or_else(|p| p.into_inner());
        let state = upgrades
            .iter()
            .find(|u| {
                u.request.id == id
                    && u.request.device.id == device.id
                    && u.request.expires_at > now()
            })
            .map_or("expired".into(), |u| u.state.clone());
        Ok(ConnectionProgress {
            api_version: 2,
            state,
        })
    }
}
#[cfg(test)]
mod connection_tests {
    use super::*;
    struct Fixture {
        path: PathBuf,
        sessions: Sessions,
    }
    impl Fixture {
        fn new() -> Self {
            let directory =
                std::env::temp_dir().join(format!("webdeck-connections-{}", domain::id().unwrap()));
            fs::create_dir(&directory).unwrap();
            let path = directory.join("devices.v2.json");
            Self {
                sessions: Sessions::open(path.clone()).unwrap(),
                path,
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(self.path.parent().unwrap());
        }
    }
    fn grant() -> DeviceRequest {
        DeviceRequest {
            name: "Phone".into(),
            capabilities: vec![Capability::Read],
            ttl_seconds: 2592000,
        }
    }
    #[test]
    fn invitation_is_short_lived_one_time_and_not_authorization() {
        let f = Fixture::new();
        let (invitation, expires) = f.sessions.invitation().unwrap();
        assert!(expires <= now() + 120);
        assert!(f.sessions.authorize(Some(&invitation), false).is_err());
        let ip = "192.168.1.2".parse().unwrap();
        let challenge = f
            .sessions
            .invited_pairing(&invitation, "Phone".into(), ip)
            .unwrap();
        assert_eq!(challenge.code.len(), 6);
        assert!(f
            .sessions
            .invited_pairing(&invitation, "Replay".into(), ip)
            .is_err());
        let (expired, _) = f.sessions.invitation().unwrap();
        f.sessions
            .invitations
            .lock()
            .unwrap()
            .last_mut()
            .unwrap()
            .expires_at = now();
        assert!(f.sessions.validate_invitation(&expired).is_err());
    }
    #[test]
    fn approval_delivery_is_single_use_bound_to_secret_and_source() {
        let f = Fixture::new();
        let ip = "192.168.1.2".parse().unwrap();
        let challenge = f.sessions.request_pairing("Phone".into(), ip).unwrap();
        let claim = PairingClaim {
            id: challenge.id.clone(),
            secret: challenge.secret.clone(),
        };
        assert!(f
            .sessions
            .claim_pairing(claim.clone(), "192.168.1.3".parse().unwrap())
            .is_err());
        assert!(f
            .sessions
            .claim_pairing(
                PairingClaim {
                    id: challenge.id.clone(),
                    secret: "wrong".into()
                },
                ip
            )
            .is_err());
        let mut invalid = grant();
        invalid.name.clear();
        assert!(f.sessions.accept_pairing(&challenge.id, invalid).is_err());
        let mut renamed = grant();
        renamed.name = "Living room phone".into();
        f.sessions.accept_pairing(&challenge.id, renamed).unwrap();
        assert_eq!(f.sessions.list()[0].name, "Living room phone");
        assert!(f.sessions.accept_pairing(&challenge.id, grant()).is_err());
        let result = f.sessions.claim_pairing(claim.clone(), ip).unwrap();
        assert_eq!(result.state, "approved");
        assert!(!result.token.is_empty());
        assert_eq!(
            f.sessions.authorize(Some(&result.token), false).unwrap(),
            vec![Capability::Read]
        );
        let replay = f.sessions.claim_pairing(claim, ip).unwrap();
        assert_eq!(replay.state, "expired");
        assert!(replay.token.is_empty());
    }
    #[test]
    fn rejected_and_expired_pairings_never_deliver_credentials() {
        let f = Fixture::new();
        let ip = "192.168.1.2".parse().unwrap();
        let challenge = f.sessions.request_pairing("Phone".into(), ip).unwrap();
        f.sessions.reject_pairing(&challenge.id).unwrap();
        let claim = PairingClaim {
            id: challenge.id.clone(),
            secret: challenge.secret,
        };
        let result = f.sessions.claim_pairing(claim.clone(), ip).unwrap();
        assert_eq!(result.state, "rejected");
        assert!(result.token.is_empty());
        assert!(f.sessions.accept_pairing(&challenge.id, grant()).is_err());
        f.sessions.pending.lock().unwrap()[0].request.expires_at = now();
        assert_eq!(
            f.sessions.claim_pairing(claim, ip).unwrap().state,
            "expired"
        );
    }
    #[test]
    fn trust_persists_legacy_hashes_renewal_does_not_expand_and_revocation_wins() {
        let f = Fixture::new();
        let original = f.sessions.approve(grant()).unwrap();
        let bytes = fs::read(&f.path).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains(&original.token));
        let reloaded = Sessions::open(f.path.clone()).unwrap();
        assert_eq!(
            reloaded.authorize(Some(&original.token), false).unwrap(),
            vec![Capability::Read]
        );
        reloaded.renew(&original.device.id, 315360000).unwrap();
        assert_eq!(
            reloaded.identify(&original.token).unwrap().expires_at,
            9007199254740991
        );
        assert_eq!(
            reloaded.identify(&original.token).unwrap().capabilities,
            vec![Capability::Read]
        );
        reloaded.revoke(&original.device.id).unwrap();
        assert_eq!(
            f.sessions
                .authorize(Some(&original.token), false)
                .unwrap_err()
                .code,
            ErrorCode::DeviceRevoked
        );
        assert!(reloaded.renew(&original.device.id, 2592000).is_err());
        assert!(reloaded
            .update(
                &original.device.id,
                DeviceUpdate {
                    name: "Renamed".into(),
                    capabilities: vec![Capability::Read],
                    ttl_seconds: 900
                }
            )
            .is_err());
    }
    #[test]
    fn permission_requests_require_explicit_decision_and_cannot_grant_admin() {
        let f = Fixture::new();
        let device = f.sessions.approve(grant()).unwrap();
        assert!(f
            .sessions
            .request_permissions(&device.token, vec![Capability::Admin])
            .is_err());
        assert!(f
            .sessions
            .request_permissions(&device.token, vec![Capability::Settings])
            .is_err());
        let request = f
            .sessions
            .request_permissions(&device.token, vec![Capability::Input])
            .unwrap();
        assert!(!f
            .sessions
            .authorize(Some(&device.token), false)
            .unwrap()
            .contains(&Capability::Input));
        f.sessions.decide_permissions(&request.id, false).unwrap();
        assert!(!f
            .sessions
            .authorize(Some(&device.token), false)
            .unwrap()
            .contains(&Capability::Input));
        let request = f
            .sessions
            .request_permissions(&device.token, vec![Capability::Input])
            .unwrap();
        f.sessions.decide_permissions(&request.id, true).unwrap();
        assert!(f
            .sessions
            .authorize(Some(&device.token), false)
            .unwrap()
            .contains(&Capability::Input));
        assert_eq!(
            f.sessions.identify(&device.token).unwrap().expires_at,
            device.device.expires_at
        );
        assert!(f.sessions.decide_permissions(&request.id, true).is_err());
        let request = f
            .sessions
            .request_permissions(&device.token, vec![Capability::Power])
            .unwrap();
        f.sessions.revoke(&device.device.id).unwrap();
        assert!(f.sessions.decide_permissions(&request.id, true).is_err());
    }
    #[test]
    fn presence_counts_multiple_connections_and_history_survives_restart() {
        let f = Fixture::new();
        let device = f.sessions.approve(grant()).unwrap();
        assert!(!f.sessions.device_presence().devices[0].online);
        f.sessions.connected(&device.token).unwrap();
        f.sessions.connected(&device.token).unwrap();
        f.sessions.disconnected(&device.device.id);
        assert!(f.sessions.device_presence().devices[0].online);
        f.sessions.disconnected(&device.device.id);
        f.sessions.persist_history().unwrap();
        let restarted = Sessions::open(f.path.clone()).unwrap();
        assert!(!restarted.device_presence().devices[0].online);
        assert!(restarted.device_presence().devices[0].last_seen > 0);
    }
    #[test]
    fn concurrent_invitations_and_claims_have_one_winner() {
        let f = Fixture::new();
        let sessions = &f.sessions;
        let (invitation, _) = sessions.invitation().unwrap();
        let ip = "192.168.1.2".parse().unwrap();
        let results = std::thread::scope(|scope| {
            let a = scope.spawn(|| sessions.invited_pairing(&invitation, "Phone A".into(), ip));
            let b = scope.spawn(|| sessions.invited_pairing(&invitation, "Phone B".into(), ip));
            [a.join().unwrap(), b.join().unwrap()]
        });
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        let challenge = results.into_iter().find_map(Result::ok).unwrap();
        sessions.accept_pairing(&challenge.id, grant()).unwrap();
        let claim = PairingClaim {
            id: challenge.id,
            secret: challenge.secret,
        };
        let results = std::thread::scope(|scope| {
            let a = scope.spawn(|| sessions.claim_pairing(claim.clone(), ip));
            let b = scope.spawn(|| sessions.claim_pairing(claim.clone(), ip));
            [a.join().unwrap().unwrap(), b.join().unwrap().unwrap()]
        });
        assert_eq!(results.iter().filter(|r| !r.token.is_empty()).count(), 1);
    }
    #[test]
    fn invalid_claims_are_rate_limited_and_pending_resources_bounded() {
        let f = Fixture::new();
        let ip = "192.168.1.2".parse().unwrap();
        for _ in 0..180 {
            assert!(f
                .sessions
                .claim_pairing(
                    PairingClaim {
                        id: "invalid".into(),
                        secret: "invalid".into()
                    },
                    ip
                )
                .is_err());
        }
        assert_eq!(
            f.sessions
                .claim_pairing(
                    PairingClaim {
                        id: "invalid".into(),
                        secret: "invalid".into()
                    },
                    ip
                )
                .unwrap_err()
                .code,
            ErrorCode::CapacityExhausted
        );
        for n in 1..=32 {
            f.sessions
                .request_pairing("Phone".into(), format!("192.168.2.{n}").parse().unwrap())
                .unwrap();
        }
        assert_eq!(
            f.sessions
                .request_pairing("Phone".into(), "192.168.3.1".parse().unwrap())
                .unwrap_err()
                .code,
            ErrorCode::CapacityExhausted
        );
    }
    #[test]
    fn expiration_is_structured_and_host_renewal_preserves_the_credential_and_grant() {
        let f = Fixture::new();
        let grant = f.sessions.approve(grant()).unwrap();
        let mut stored: serde_json::Value =
            serde_json::from_slice(&fs::read(&f.path).unwrap()).unwrap();
        stored[0]["device"]["expires_at"] = serde_json::json!(0);
        storage::atomic_replace(&f.path, &serde_json::to_vec(&stored).unwrap()).unwrap();
        assert_eq!(
            f.sessions
                .authorize(Some(&grant.token), false)
                .unwrap_err()
                .code,
            ErrorCode::AuthorizationExpired
        );
        assert_eq!(
            f.sessions.identify(&grant.token).unwrap_err().code,
            ErrorCode::AuthorizationExpired
        );
        f.sessions.renew(&grant.device.id, 2592000).unwrap();
        assert_eq!(
            f.sessions.authorize(Some(&grant.token), false).unwrap(),
            vec![Capability::Read]
        );
    }
    #[test]
    fn permission_progress_is_private_and_retains_terminal_decisions_for_polling() {
        let f = Fixture::new();
        let first = f.sessions.approve(grant()).unwrap();
        let second = f.sessions.approve(grant()).unwrap();
        let request = f
            .sessions
            .request_permissions(&first.token, vec![Capability::Audio])
            .unwrap();
        assert_eq!(
            f.sessions
                .permission_progress(&first.token, &request.id)
                .unwrap()
                .state,
            "pending"
        );
        assert_eq!(
            f.sessions
                .permission_progress(&second.token, &request.id)
                .unwrap()
                .state,
            "expired"
        );
        f.sessions.decide_permissions(&request.id, false).unwrap();
        assert_eq!(
            f.sessions
                .permission_progress(&first.token, &request.id)
                .unwrap()
                .state,
            "rejected"
        );
        assert!(f.sessions.permission_requests().requests.is_empty());
        let next = f
            .sessions
            .request_permissions(&first.token, vec![Capability::Audio])
            .unwrap();
        assert_ne!(next.id, request.id);
        f.sessions.decide_permissions(&next.id, true).unwrap();
        assert_eq!(
            f.sessions
                .permission_progress(&first.token, &next.id)
                .unwrap()
                .state,
            "approved"
        );
        assert!(f.sessions.decide_permissions(&next.id, true).is_err());
        f.sessions
            .upgrades
            .lock()
            .unwrap()
            .last_mut()
            .unwrap()
            .request
            .expires_at = now();
        assert_eq!(
            f.sessions
                .permission_progress(&first.token, &next.id)
                .unwrap()
                .state,
            "expired"
        );
    }
}
