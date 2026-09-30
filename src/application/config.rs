use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::adapters::config::{atomic_replace, protected_write};
use crate::domain::config::{ConfigDocument, SCHEMA_VERSION};
use crate::domain::error::{AppError, ErrorCode};

#[derive(Clone, Debug, Serialize)]
pub struct ConfigSnapshot {
    pub revision: u64,
    pub config: Value,
}

struct State {
    snapshot: ConfigSnapshot,
    fingerprint: Vec<u8>,
}

/// One owner per configuration path. The lock spans only synchronous config
/// operations, never unrelated integration/network work.
pub struct ConfigService {
    path: PathBuf,
    state: Mutex<State>,
}

fn read_document(path: &Path) -> Result<(ConfigDocument, Vec<u8>), AppError> {
    let bytes = fs::read(path)
        .map_err(|_| AppError::new(ErrorCode::PersistenceFailed, "Cannot read configuration"))?;
    let value = serde_json::from_slice(&bytes).map_err(|_| {
        AppError::new(
            ErrorCode::InvalidInput,
            "Configuration contains invalid JSON",
        )
    })?;
    Ok((ConfigDocument::validate(value)?, bytes))
}

impl ConfigService {
    pub fn open(path: PathBuf) -> Result<Self, AppError> {
        let _writer = crate::adapters::config::exclusive_lock(&path)?;
        let (document, mut bytes) = read_document(&path)?;
        let migrated = document.clone().migrate();
        if migrated != document {
            let backup = path.with_extension("v1.backup.json");
            if !backup.exists() {
                protected_write(&backup, &bytes, true)?;
            }
            bytes = serde_json::to_vec_pretty(&migrated.0)
                .map_err(|_| AppError::new(ErrorCode::InvalidInput, "Invalid configuration"))?;
            atomic_replace(&path, &bytes)?;
        }
        Ok(Self {
            path,
            state: Mutex::new(State {
                snapshot: ConfigSnapshot {
                    revision: u64::from_le_bytes(Sha256::digest(&bytes)[..8].try_into().unwrap())
                        & ((1u64 << 48) - 1),
                    config: migrated.0,
                },
                fingerprint: bytes,
            }),
        })
    }

    fn reload(&self, state: &mut State) -> Result<(), AppError> {
        let bytes = fs::read(&self.path).map_err(|_| {
            AppError::new(ErrorCode::PersistenceFailed, "Cannot read configuration")
        })?;
        if bytes == state.fingerprint {
            return Ok(());
        }
        let value = serde_json::from_slice(&bytes).map_err(|_| {
            AppError::new(
                ErrorCode::InvalidInput,
                "External configuration contains invalid JSON",
            )
        })?;
        let document = ConfigDocument::validate(value)?;
        // External edits are validated, but never rewritten during reads.
        state.snapshot.config = document.0;
        state.snapshot.revision += 1;
        state.fingerprint = bytes;
        Ok(())
    }

    pub fn snapshot(&self) -> Result<ConfigSnapshot, AppError> {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        self.reload(&mut state)?;
        Ok(state.snapshot.clone())
    }

    pub fn last_valid(&self) -> ConfigSnapshot {
        self.state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .snapshot
            .clone()
    }

    /// A missing revision is reserved for legacy last-write-wins callers.
    pub fn update(
        &self,
        expected: Option<u64>,
        apply: impl FnOnce(Value) -> Result<Value, AppError>,
    ) -> Result<ConfigSnapshot, AppError> {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let _writer = crate::adapters::config::exclusive_lock(&self.path)?;
        self.reload(&mut state)?;
        if expected.is_some_and(|rev| rev != state.snapshot.revision) {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "Configuration changed; reload before saving",
            ));
        }
        let value = ConfigDocument::validate(apply(state.snapshot.config.clone())?)?
            .migrate()
            .0;
        if value == state.snapshot.config {
            return Ok(state.snapshot.clone());
        }
        let bytes = serde_json::to_vec_pretty(&value)
            .map_err(|_| AppError::new(ErrorCode::InvalidInput, "Invalid configuration"))?;
        // Detect external modifications made while the transaction ran.
        if fs::read(&self.path).ok().as_deref() != Some(state.fingerprint.as_slice()) {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "Configuration changed during save",
            ));
        }
        atomic_replace(&self.path, &bytes)?;
        state.fingerprint = bytes;
        state.snapshot = ConfigSnapshot {
            revision: state.snapshot.revision + 1,
            config: value,
        };
        Ok(state.snapshot.clone())
    }

    pub fn replace(&self, value: Value, expected: Option<u64>) -> Result<ConfigSnapshot, AppError> {
        self.update(expected, |_| Ok(value))
    }

    pub fn restore_backup(&self, expected: u64) -> Result<ConfigSnapshot, AppError> {
        let (backup, _) = read_document(&self.path.with_extension("v1.backup.json"))?;
        self.replace(backup.migrate().0, Some(expected))
    }
}

pub fn shared(path: PathBuf) -> Result<Arc<ConfigService>, AppError> {
    static SERVICES: OnceLock<Mutex<HashMap<PathBuf, Arc<ConfigService>>>> = OnceLock::new();
    let mut services = SERVICES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    if let Some(service) = services.get(&path) {
        return Ok(service.clone());
    }
    let service = Arc::new(ConfigService::open(path.clone())?);
    services.insert(path, service.clone());
    Ok(service)
}

pub const CURRENT_SCHEMA_VERSION: u64 = SCHEMA_VERSION;
