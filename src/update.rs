//! Verified, confined staging and recoverable installation. All filesystem
//! tests use disposable trees; production never installs unverified bytes.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    io::{self},
    path::{Component, Path, PathBuf},
};
const MAX_ARCHIVE: usize = 256 * 1024 * 1024;
const MAX_EXPANDED: u64 = 1024 * 1024 * 1024;
fn invalid() -> io::Error {
    io::Error::other("Invalid or unverified update artifact")
}
pub fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub(crate) fn relative(name: &str) -> io::Result<PathBuf> {
    if name.contains(['\\', ':']) || name.chars().any(char::is_control) {
        return Err(invalid());
    }
    let path = Path::new(name);
    if path
        .components()
        .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(invalid());
    }
    let first = path
        .components()
        .next()
        .ok_or_else(invalid)?
        .as_os_str()
        .to_str()
        .ok_or_else(invalid)?;
    if !matches!(
        first,
        "WebDeck"
            | "WebDeck.exe"
            | "console"
            | "console.exe"
            | "update"
            | "update.exe"
            | "webdeck-qr"
            | "webdeck-qr.exe"
            | "webdeck"
            | "static"
            | "frontend"
            | "docs"
            | "lib"
    ) {
        return Err(invalid());
    }
    if path.components().any(|c| {
        c.as_os_str().to_str().is_none_or(|s| {
            s.starts_with('.') || s.ends_with(['.', ' ']) || {
                let stem = s.split('.').next().unwrap_or("").to_ascii_uppercase();
                matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                    || stem
                        .strip_prefix("COM")
                        .or_else(|| stem.strip_prefix("LPT"))
                        .is_some_and(|number| {
                            matches!(number, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
                        })
            }
        })
    }) {
        return Err(invalid());
    }
    Ok(path.to_owned())
}
pub(crate) fn confined(root: &Path, rel: &Path) -> io::Result<PathBuf> {
    let mut target = root.to_path_buf();
    for part in rel.components() {
        if !matches!(part, Component::Normal(_)) {
            return Err(invalid());
        }
        target.push(part.as_os_str());
        if fs::symlink_metadata(&target).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(invalid());
        }
    }
    Ok(target)
}
/// Stage into a fresh, private directory. Reject duplicates, traversal,
/// links, config replacement and zip bombs before any live installation.
pub fn stage(bytes: &[u8], expected: &str, directory: &Path) -> io::Result<PathBuf> {
    let expected = expected.strip_prefix("sha256:").ok_or_else(invalid)?;
    if bytes.len() > MAX_ARCHIVE || expected.len() != 64 || digest(bytes) != expected {
        return Err(invalid());
    }
    fs::create_dir(directory)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700))?;
    }
    let result = (|| {
        let mut archive =
            zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|_| invalid())?;
        if archive.len() > 10000 {
            return Err(invalid());
        }
        let mut seen = HashSet::new();
        let mut expanded = 0u64;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|_| invalid())?;
            let raw = entry.name().strip_prefix("WebDeck/").ok_or_else(invalid)?;
            if raw.is_empty() && entry.is_dir() {
                continue;
            }
            let rel = relative(raw.trim_end_matches('/'))?;
            if !seen.insert(rel.to_string_lossy().to_lowercase())
                || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
            {
                return Err(invalid());
            }
            expanded = expanded.checked_add(entry.size()).ok_or_else(invalid)?;
            if expanded > MAX_EXPANDED {
                return Err(invalid());
            }
            let path = directory.join(rel);
            if entry.is_dir() {
                fs::create_dir_all(path)?;
                continue;
            }
            fs::create_dir_all(path.parent().ok_or_else(invalid)?)?;
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            if io::copy(&mut entry, &mut output)? != entry.size() {
                return Err(invalid());
            }
            output.sync_all()?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = if entry.unix_mode().unwrap_or(0) & 0o111 != 0 {
                    0o755
                } else {
                    0o644
                };
                fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
            }
        }
        #[cfg(windows)]
        let executable = "WebDeck.exe";
        #[cfg(not(windows))]
        let executable = "WebDeck";
        for required in [
            executable,
            "webdeck/config_default.json",
            "webdeck/version.json",
            "frontend/dist/index.html",
        ] {
            if !directory.join(required).is_file() {
                return Err(invalid());
            }
        }
        crate::domain::decode_config(&fs::read(directory.join("webdeck/config_default.json"))?)
            .map_err(|_| invalid())?;
        let version: serde_json::Value =
            serde_json::from_slice(&fs::read(directory.join("webdeck/version.json"))?)
                .map_err(|_| invalid())?;
        let version = version["versions"][0]["version"]
            .as_str()
            .ok_or_else(invalid)?;
        let version = semver::Version::parse(version).map_err(|_| invalid())?;
        if version.major != 2 || version.pre.is_empty() {
            return Err(invalid());
        }
        Ok(directory.to_path_buf())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(directory);
    }
    result
}
#[derive(Serialize, Deserialize)]
struct Entry {
    path: String,
    existed: bool,
}
fn write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    crate::storage::atomic_replace(path, bytes)
        .map_err(|_| io::Error::other("Cannot install update file"))
}
fn paths(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                return Err(invalid());
            }
            if kind.is_dir() {
                stack.push(entry.path());
            } else if kind.is_file() {
                files.push(
                    entry
                        .path()
                        .strip_prefix(root)
                        .map_err(|_| invalid())?
                        .to_path_buf(),
                );
            }
        }
    }
    files.sort();
    Ok(files)
}
/// A durable journal + originals permit recovery after interruption. Commit
/// keeps the backup until an explicit rollback or a later operator cleanup.
pub fn install(staged: &Path, destination: &Path, backup: &Path) -> io::Result<()> {
    install_with(staged, destination, backup, |_, _| Ok(()))
}
pub fn install_with(
    staged: &Path,
    destination: &Path,
    backup: &Path,
    mut before_write: impl FnMut(usize, &Path) -> io::Result<()>,
) -> io::Result<()> {
    fs::create_dir(backup)?;
    let mut entries = Vec::new();
    // Finish the entire backup and journal before replacing any live file.
    for rel in paths(staged)? {
        let rel = relative(&rel.to_string_lossy().replace('\\', "/"))?;
        let live = confined(destination, &rel)?;
        let existed = live.exists();
        if existed {
            let original = backup.join("original").join(&rel);
            fs::create_dir_all(original.parent().ok_or_else(invalid)?)?;
            fs::copy(&live, &original)?;
            fs::File::open(original)?.sync_all()?;
        }
        entries.push(Entry {
            path: rel.to_string_lossy().replace('\\', "/"),
            existed,
        });
    }
    write(&backup.join("journal.json"), &serde_json::to_vec(&entries)?)?;
    let result = (|| {
        for (i, entry) in entries.iter().enumerate() {
            let rel = Path::new(&entry.path);
            let live = confined(destination, rel)?;
            fs::create_dir_all(live.parent().ok_or_else(invalid)?)?;
            before_write(i, &live)?;
            write(&live, &fs::read(staged.join(rel))?)?;
            fs::set_permissions(&live, fs::metadata(staged.join(rel))?.permissions())?;
        }
        write(&backup.join("committed"), b"verified installation complete")
    })();
    if let Err(error) = result {
        rollback(destination, backup)?;
        return Err(error);
    }
    Ok(())
}
pub fn rollback(destination: &Path, backup: &Path) -> io::Result<()> {
    let entries: Vec<Entry> = serde_json::from_slice(&fs::read(backup.join("journal.json"))?)?;
    for entry in entries.into_iter().rev() {
        let rel = relative(&entry.path)?;
        let live = confined(destination, &rel)?;
        if entry.existed {
            let original = backup.join("original").join(&rel);
            write(&live, &fs::read(&original)?)?;
            fs::set_permissions(&live, fs::metadata(original)?.permissions())?;
        } else if live.exists() {
            fs::remove_file(live)?;
        }
    }
    write(&backup.join("rolled-back"), b"original files restored")
}
/// Download caps and timeouts are independent of archive expansion limits.
pub async fn download(url: &str) -> io::Result<Vec<u8>> {
    let parsed = reqwest::Url::parse(url).map_err(|_| invalid())?;
    if !trusted_artifact(&parsed) {
        return Err(invalid());
    }
    let client = reqwest::Client::builder()
        .user_agent("WebDeck")
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|_| invalid())?;
    let mut response = client
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|_| invalid())?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| invalid())? {
        if bytes.len() + chunk.len() > MAX_ARCHIVE {
            return Err(invalid());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

pub fn trusted_artifact(url: &reqwest::Url) -> bool {
    if url.scheme() != "https"
        || url.host_str() != Some("github.com")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return false;
    }
    let parts = url.path().split('/').collect::<Vec<_>>();
    if parts.len() != 7
        || parts[1] != "nglmercer"
        || parts[2] != "WebDeck"
        || parts[3] != "releases"
        || parts[4] != "download"
    {
        return false;
    }
    let Some(version) = parts[5]
        .strip_prefix('v')
        .and_then(|v| semver::Version::parse(v).ok())
    else {
        return false;
    };
    if version.major != 2 || version.pre.is_empty() {
        return false;
    }
    let platform = package_platform();
    parts[6]
        == format!(
            "WebDeck-{version}-{platform}-{}-portable.zip",
            std::env::consts::ARCH
        )
}
/// Native platform label shared by portable packages and verified update URLs.
pub const fn package_platform() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Release {
    pub version: String,
    pub url: String,
    pub digest: String,
}
pub async fn latest() -> io::Result<Option<Release>> {
    let response = reqwest::Client::builder()
        .user_agent("WebDeck v2")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|_| invalid())?
        .get("https://api.github.com/repos/nglmercer/WebDeck/releases?per_page=100")
        .send()
        .await
        .map_err(|_| invalid())?
        .error_for_status()
        .map_err(|_| invalid())?;
    let releases: Vec<serde_json::Value> = response.json().await.map_err(|_| invalid())?;
    select_release(&releases)
}
pub fn select_release(releases: &[serde_json::Value]) -> io::Result<Option<Release>> {
    let current = semver::Version::parse(env!("CARGO_PKG_VERSION")).map_err(|_| invalid())?;
    let mut candidates = Vec::new();
    for r in releases {
        if r["draft"] != false || r["prerelease"] != true {
            continue;
        }
        let Some(version) = r["tag_name"]
            .as_str()
            .and_then(|s| s.strip_prefix('v'))
            .and_then(|s| semver::Version::parse(s).ok())
        else {
            continue;
        };
        if version.major != 2 || version.pre.is_empty() || version <= current {
            continue;
        }
        if let Some(assets) = r["assets"].as_array() {
            for a in assets {
                let Some(url) = a["browser_download_url"]
                    .as_str()
                    .and_then(|s| reqwest::Url::parse(s).ok())
                else {
                    continue;
                };
                let Some(digest) = a["digest"].as_str() else {
                    continue;
                };
                if !trusted_artifact(&url)
                    || !url.path().contains(&format!("/v{version}/"))
                    || !digest
                        .strip_prefix("sha256:")
                        .is_some_and(|d| d.len() == 64 && d.bytes().all(|b| b.is_ascii_hexdigit()))
                {
                    continue;
                }
                candidates.push((
                    version.clone(),
                    Release {
                        version: version.to_string(),
                        url: url.to_string(),
                        digest: digest.to_string(),
                    },
                ));
            }
        }
    }
    candidates.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(candidates.pop().map(|(_, r)| r))
}
