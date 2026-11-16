//! Reloads plugins when their folder changes. Polling needs no extra dependency. A change is
//! applied only after one full interval with the same fingerprint, so a package that is still
//! being copied in is not read half-written.
use super::*;
use std::{
    path::Path,
    time::{Duration, UNIX_EPOCH},
};

const INTERVAL: Duration = Duration::from_secs(2);

/// Watches `<config>/plugins` for the life of the server.
pub fn spawn(app: App) {
    tokio::spawn(async move {
        let mut applied = fingerprint(&app.assets.root.join("plugins"));
        let mut seen: Option<String> = None;
        loop {
            tokio::time::sleep(INTERVAL).await;
            let current = fingerprint(&app.assets.root.join("plugins"));
            if current == applied {
                seen = None;
                continue;
            }
            if seen.as_deref() != Some(current.as_str()) {
                seen = Some(current);
                continue;
            }
            match reload_runtime(&app).await {
                Ok(_) => applied = current,
                // Keep the old fingerprint so the next tick tries again.
                Err(error) => eprintln!("WebDeck: automatic plugin reload failed: {error}"),
            }
            seen = None;
        }
    });
}

/// A digest of the plugin folder: every file of every package with its size and modification
/// time. A new, removed or edited file changes it.
pub fn fingerprint(root: &Path) -> String {
    let Ok(packages) = std::fs::read_dir(root) else {
        return String::from("missing");
    };
    let mut entries = Vec::new();
    for package in packages.flatten() {
        let name = package.file_name().to_string_lossy().into_owned();
        entries.push(name.clone());
        let Ok(files) = std::fs::read_dir(package.path()) else {
            continue;
        };
        for file in files.flatten() {
            let Ok(metadata) = file.metadata() else {
                continue;
            };
            let modified = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |time| time.as_nanos());
            entries.push(format!(
                "{name}/{}:{}:{modified}",
                file.file_name().to_string_lossy(),
                metadata.len()
            ));
        }
    }
    entries.sort();
    entries.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_changes_when_a_package_file_changes() {
        let root =
            std::env::temp_dir().join(format!("webdeck-watch-{}", crate::domain::id().unwrap()));
        let package = root.join("echo");
        std::fs::create_dir_all(&package).unwrap();
        std::fs::write(package.join("index.js"), "a").unwrap();
        let first = fingerprint(&root);
        assert_eq!(first, fingerprint(&root));
        std::fs::write(package.join("index.js"), "changed bytes").unwrap();
        assert_ne!(first, fingerprint(&root));
        std::fs::remove_dir_all(&package).unwrap();
        assert!(!fingerprint(&root).contains("echo"));
        std::fs::remove_dir_all(&root).unwrap();
    }
}
