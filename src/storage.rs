use crate::{
    contracts::*,
    domain::{self, Error, Result},
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::Mutex,
};
fn failed() -> Error {
    Error::new(
        ErrorCode::PersistenceFailed,
        "Cannot persist application data",
    )
}
pub fn protected_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut o = OpenOptions::new();
    o.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let mut f = o.open(path).map_err(|_| failed())?;
    f.write_all(bytes).map_err(|_| failed())?;
    f.sync_all().map_err(|_| failed())
}
pub fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<()> {
    let t = path.with_extension(format!("{}.tmp", domain::id()?));
    let result = (|| {
        protected_write(&t, bytes)?;
        fs::rename(&t, path).map_err(|_| failed())?;
        #[cfg(unix)]
        {
            if let Some(p) = path.parent() {
                if let Ok(f) = fs::File::open(p) {
                    let _ = f.sync_all();
                }
            }
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(t);
    }
    result
}
pub fn lock(path: &Path) -> Result<fs::File> {
    let mut o = OpenOptions::new();
    o.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let f = o.open(path.with_extension("lock")).map_err(|_| failed())?;
    f.lock().map_err(|_| failed())?;
    Ok(f)
}
fn read(path: &Path) -> Result<Vec<u8>> {
    let mut o = OpenOptions::new();
    o.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.custom_flags(libc::O_NOFOLLOW);
    }
    let mut b = Vec::new();
    o.open(path)
        .map_err(|_| failed())?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut b)
        .map_err(|_| failed())?;
    if b.len() > 16 * 1024 * 1024 {
        return Err(failed());
    }
    Ok(b)
}
fn revision(b: &[u8]) -> u64 {
    u64::from_le_bytes(Sha256::digest(b)[..8].try_into().expect("digest size")) & ((1u64 << 48) - 1)
}
#[derive(Clone)]
struct State {
    snapshot: ConfigResponse,
    bytes: Vec<u8>,
}
pub struct ConfigStore {
    path: PathBuf,
    state: Mutex<State>,
}
impl ConfigStore {
    pub fn open(path: PathBuf) -> Result<Self> {
        if let Some(p) = path.parent() {
            fs::create_dir_all(p).map_err(|_| failed())?;
        }
        let _l = lock(&path)?;
        if !path.exists() {
            if Path::new("config.json").exists() {
                return Err(Error::new(
                    ErrorCode::UnsupportedSchema,
                    "Existing root configuration requires manual recovery; files preserved",
                ));
            }
            let b = include_bytes!("../webdeck/config_default.json");
            domain::decode_config(b)?;
            protected_write(&path, b)?;
        }
        let b = read(&path)?;
        let c = domain::decode_config(&b).map_err(|e| {
            Error::new(
                e.code,
                format!(
                    "Cannot load {}: {}. File preserved; to start with v2 defaults, move this file to a backup and restart, or use --config-dir with an empty directory",
                    path.display(), e.message
                ),
            )
        })?;
        Ok(Self {
            path,
            state: Mutex::new(State {
                snapshot: ConfigResponse {
                    api_version: 2,
                    revision: revision(&b),
                    config: c,
                },
                bytes: b,
            }),
        })
    }
    fn reload(&self, s: &mut State) -> Result<()> {
        let b = read(&self.path)?;
        if b != s.bytes {
            let c = domain::decode_config(&b)?;
            s.snapshot = ConfigResponse {
                api_version: 2,
                revision: revision(&b),
                config: c,
            };
            s.bytes = b;
        }
        Ok(())
    }
    pub fn snapshot(&self) -> Result<ConfigResponse> {
        let mut s = self.state.lock().unwrap_or_else(|p| p.into_inner());
        self.reload(&mut s)?;
        Ok(s.snapshot.clone())
    }
    pub fn last_valid(&self) -> ConfigResponse {
        self.state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .snapshot
            .clone()
    }
    pub fn mutate(
        &self,
        expected: u64,
        f: impl FnOnce(&mut Config) -> Result<()>,
    ) -> Result<ConfigResponse> {
        if expected > 9007199254740991 {
            return Err(Error::invalid());
        }
        let mut s = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let _l = lock(&self.path)?;
        self.reload(&mut s)?;
        if expected != s.snapshot.revision {
            return Err(Error::new(
                ErrorCode::Conflict,
                "Configuration changed; your draft has been preserved",
            ));
        }
        let mut c = s.snapshot.config.clone();
        f(&mut c)?;
        domain::validate_config(&c)?;
        if c == s.snapshot.config {
            return Ok(s.snapshot.clone());
        }
        let b = serde_json::to_vec_pretty(&c).map_err(|_| failed())?;
        if read(&self.path)? != s.bytes {
            return Err(Error::new(
                ErrorCode::Conflict,
                "Configuration changed during save",
            ));
        }
        atomic_replace(&self.path, &b)?;
        s.snapshot = ConfigResponse {
            api_version: 2,
            revision: revision(&b),
            config: c,
        };
        s.bytes = b;
        Ok(s.snapshot.clone())
    }
}
#[derive(Clone)]
pub struct Assets {
    pub root: PathBuf,
}
impl Assets {
    pub fn path(&self, id: &str) -> Result<PathBuf> {
        if id.is_empty()
            || id.len() > 128
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
            || matches!(id, "." | "..")
        {
            return Err(Error::invalid());
        }
        let root = self.root.join("user_uploads");
        fs::create_dir_all(&root).map_err(|_| failed())?;
        if fs::symlink_metadata(&root).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::invalid());
        }
        let p = root.join(id);
        if fs::symlink_metadata(&p).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::invalid());
        }
        Ok(p)
    }
    pub fn resolve(&self, s: &FileSource) -> Result<PathBuf> {
        match s {
            FileSource::Asset { id } => self.path(id),
            FileSource::External { path } => {
                let p = PathBuf::from(path);
                if !p.is_absolute() || p.components().any(|c| matches!(c, Component::ParentDir)) {
                    return Err(Error::invalid());
                }
                Ok(p)
            }
        }
    }
    pub fn read(&self, id: &str) -> Result<Vec<u8>> {
        read(&self.path(id)?)
    }
    pub fn upload(&self, extension: &str, bytes: &[u8]) -> Result<FileSource> {
        let allowed = [
            "png", "jpg", "jpeg", "webp", "gif", "svg", "mp3", "wav", "ogg", "flac", "css", "rhai",
            "txt",
        ];
        if !allowed.contains(&extension) || bytes.len() > 16 * 1024 * 1024 {
            return Err(Error::invalid());
        }
        let id = format!("{}.{}", domain::id()?, extension);
        protected_write(&self.path(&id)?, bytes)?;
        Ok(FileSource::Asset { id })
    }
}
