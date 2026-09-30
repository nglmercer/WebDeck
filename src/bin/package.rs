//! Portable-packaging tool — replacement for `setup.py` + `build.bat`.
//!
//! Produces the `*portable.zip` artifact the updater consumes (top-level
//! `WebDeck/` folder): release binaries plus the runtime data tree.
//! Run with `cargo run --release --bin package`.
//!
//! Mapping vs `setup.py`:
//! - cx_Freeze `build_exe` → `cargo build --release` (`webdeck` → `WebDeck`,
//!   `update` → `update`, `webdeck-qr` → `webdeck-qr`; the dev-only
//!   `console` binary is not shipped, like before).
//! - `include_files` (whole repo minus ignores) → the runtime-closed set:
//!   `webdeck/`, `static/`, `frontend/dist/`, docs READMEs, `lib/nircmd.exe`.
//!   No Python sources, no frozen `lib/` tree, no `venv`/`target` bloat.
//! - `frontend/dist` is built first when missing (or stale): unlike
//!   `setup.py`, whose ignore rules silently excluded it — shipping a
//!   frozen app with no web UI — it is always included here.
//! - `download_nircmd` → verified vendor ZIP; Windows requires an independent digest.
//! - `sign_executable` → same `signtool` invocation on Windows; failures
//!   fail closed for release artifacts; --dev is explicitly unsigned.
//! - `zip_build` → `dist/WebDeck-<platform>-portable.zip` with the same
//!   top-level `WebDeck/` layout the updater expects.
//! - `bdist_msi` has no cargo equivalent and stays a manual step (WiX);
//!   the portable zip is the shippable + auto-update artifact.

use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
const NIRCMD_URL: &str = "https://www.nirsoft.net/utils/nircmd.zip";
#[cfg(windows)]
const SIGNTOOL: &str = r"C:\Program Files (x86)\Windows Kits\10\bin\x64\signtool.exe";

fn main() {
    let start = std::time::Instant::now();
    if let Err(message) = package() {
        eprintln!("package: error: {message}");
        std::process::exit(1);
    }
    let elapsed = start.elapsed().as_secs();
    println!("Build done! {}m {}s", elapsed / 60, elapsed % 60);
}

fn package() -> Result<(), String> {
    let version = read_version()?;
    println!("package: WebDeck v{version}");

    release_build()?;
    ensure_frontend_dist()?;
    #[cfg(windows)]
    download_nircmd()?;

    let stage = stage_tree()?;
    sign_binaries(&stage)?;
    let zip_path = zip_stage(&stage, &version)?;
    let digest: String = Sha256::digest(std::fs::read(&zip_path).map_err(|e| e.to_string())?)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    std::fs::write(
        zip_path.with_extension("zip.sha256"),
        format!(
            "{digest}  {}\n",
            zip_path.file_name().unwrap().to_string_lossy()
        ),
    )
    .map_err(|e| e.to_string())?;

    let _ = std::fs::remove_dir_all(stage.parent().unwrap_or(Path::new("temp")));
    println!("package: portable build zipped as '{}'", zip_path.display());
    Ok(())
}

/// Port of the `webdeck/version.json` read at the top of `setup.py`.
fn read_version() -> Result<String, String> {
    let content = std::fs::read_to_string("webdeck/version.json")
        .map_err(|e| format!("cannot read webdeck/version.json: {e}"))?;
    let value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("cannot parse webdeck/version.json: {e}"))?;
    value
        .get("versions")
        .and_then(|v| v.get(0))
        .and_then(|v| v.get("version"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "webdeck/version.json has no versions[0].version".to_string())
}

/// Port of the cx_Freeze compile step: release binaries for the app + updater.
fn release_build() -> Result<(), String> {
    println!("package: cargo build --release --bin webdeck --bin update --bin webdeck-qr");
    let status = Command::new("cargo")
        .args([
            "build",
            "--locked",
            if std::env::args().any(|arg| arg == "--dev") {
                "--profile=dev"
            } else {
                "--release"
            },
            "--bin",
            "webdeck",
            "--bin",
            "update",
            "--bin",
            "webdeck-qr",
        ])
        .status()
        .map_err(|e| format!("cannot run cargo build: {e}"))?;
    if !status.success() {
        return Err(format!("cargo build failed with {status}"));
    }
    Ok(())
}

/// `setup.py` excluded the gitignored `frontend/dist` from the zip, shipping
/// a frozen app whose web UI only 500s. Build it when missing instead.
fn ensure_frontend_dist() -> Result<(), String> {
    println!("package: rebuilding frontend/dist from the current sources");
    for step in [
        vec!["--prefix", "frontend", "ci"],
        vec!["--prefix", "frontend", "run", "build"],
    ] {
        let output = Command::new("npm")
            .args(&step)
            .output()
            .map_err(|e| format!("cannot run npm {}: {e}", step.join(" ")))?;
        if !output.status.success() {
            let tail = String::from_utf8_lossy(&output.stderr);
            let tail: Vec<&str> = tail.lines().collect();
            let tail = tail.iter().rev().take(8).rev().cloned().collect::<Vec<_>>();
            return Err(format!(
                "npm {} failed:\n{}",
                step.join(" "),
                tail.join("\n")
            ));
        }
    }
    if !Path::new("frontend/dist/index.html").is_file() {
        return Err("npm build finished but frontend/dist/index.html is still missing".to_string());
    }
    Ok(())
}

/// Port of `download_nircmd` (skipped when already present, like Python).
#[cfg(windows)]
fn download_nircmd() -> Result<(), String> {
    let dest = Path::new("temp/nircmd.exe");
    let expected = std::env::var("WEBDECK_NIRCMD_SHA256").map_err(|_| {
        "WEBDECK_NIRCMD_SHA256 must pin the independently verified vendor ZIP".to_string()
    })?;
    println!("package: downloading nircmd");
    let _ = std::fs::create_dir_all("temp");
    let bytes = reqwest::blocking::get(NIRCMD_URL)
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.bytes())
        .map_err(|e| format!("cannot download nircmd: {e}"))?;
    let digest: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if expected.len() != 64 || digest != expected {
        return Err("NirCmd ZIP checksum mismatch".to_string());
    }
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| format!("bad nircmd zip: {e}"))?;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("bad nircmd entry: {e}"))?;
        let name = entry.name().to_string();
        if name.to_lowercase().ends_with("nircmd.exe") && !entry.is_dir() {
            let mut out =
                std::fs::File::create(dest).map_err(|e| format!("cannot write nircmd: {e}"))?;
            std::io::copy(&mut entry, &mut out).map_err(|e| format!("cannot write nircmd: {e}"))?;
            println!("package: staged temp/nircmd.exe");
            return Ok(());
        }
    }
    Err("nircmd.zip contained no nircmd.exe".to_string())
}

/// Copy one file, creating parent directories.
fn copy_file(src: &Path, dest: &Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("cannot stage dir: {e}"))?;
    }
    std::fs::copy(src, dest).map_err(|e| format!("cannot stage {}: {e}", src.display()))?;
    Ok(())
}

/// Recursively copy a directory tree (files only; symlinks skipped).
fn copy_tree(src: &Path, dest: &Path) -> Result<(), String> {
    let mut stack = vec![src.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries =
            std::fs::read_dir(&dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("cannot read dir entry: {e}"))?;
            let path = entry.path();
            let rel = path.strip_prefix(src).unwrap_or(&path);
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_symlink() {
                return Err("Runtime trees must not contain symlinks".to_string());
            }
            if kind.is_dir() {
                stack.push(path);
            } else if kind.is_file() {
                copy_file(&path, &dest.join(rel))?;
            }
        }
    }
    Ok(())
}

/// Stage the runtime tree into `temp/portable/WebDeck/` (port of the cx_Freeze
/// output directory that `zip_build` renames to `WebDeck`).
fn stage_tree() -> Result<PathBuf, String> {
    let root = Path::new("temp/portable");
    if root.exists() {
        let _ = std::fs::remove_dir_all(root);
    }
    let stage = root.join("WebDeck");
    let exe = std::env::consts::EXE_SUFFIX;

    for (bin, shipped) in [
        ("webdeck", "WebDeck"),
        ("update", "update"),
        ("webdeck-qr", "webdeck-qr"),
    ] {
        let src = Path::new(if std::env::args().any(|arg| arg == "--dev") {
            "target/debug"
        } else {
            "target/release"
        })
        .join(format!("{bin}{exe}"));
        if !src.is_file() {
            return Err(format!("missing release binary: {}", src.display()));
        }
        copy_file(&src, &stage.join(format!("{shipped}{exe}")))?;
    }
    for dir in ["webdeck", "static", "frontend/dist", "docs/v2"] {
        let src = Path::new(dir);
        if !src.is_dir() {
            return Err(format!("missing runtime directory: {dir}"));
        }
        copy_tree(src, &stage.join(dir))?;
    }
    // README → docs/ mapping, like `get_include_files`.
    for entry in std::fs::read_dir(".").map_err(|e| format!("cannot list repo root: {e}"))? {
        let entry = entry.map_err(|e| format!("cannot read dir entry: {e}"))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("README") && entry.path().is_file() {
            let dest = if name == "README.md" {
                "README-en.md"
            } else {
                &name
            };
            copy_file(&entry.path(), &stage.join("docs").join(dest))?;
        }
    }
    #[cfg(windows)]
    let nircmd = Path::new("temp/nircmd.exe");
    #[cfg(windows)]
    if nircmd.is_file() {
        // `setup.py` stages `nircmd*.exe` from temp/ into `lib/`.
        copy_file(nircmd, &stage.join("lib/nircmd.exe"))?;
    }
    Ok(stage)
}

/// Port of `sign_executable` (Windows only; failures warn and continue).
fn sign_binaries(stage: &Path) -> Result<(), String> {
    if std::env::args().any(|arg| arg == "--dev") {
        return Ok(());
    }
    #[cfg(windows)]
    {
        for name in ["WebDeck.exe", "update.exe", "webdeck-qr.exe"] {
            let path = stage.join(name);
            let output = Command::new(SIGNTOOL)
                .args([
                    "sign",
                    "/a",
                    "/fd",
                    "SHA256",
                    "/tr",
                    "http://timestamp.digicert.com",
                    "/td",
                    "SHA256",
                ])
                .arg(&path)
                .output();
            match output {
                Ok(output) if output.status.success() => {
                    println!("package: successfully signed {}", path.display());
                }
                Ok(output) => {
                    return Err(format!(
                        "Signing failed for {}: {}",
                        path.display(),
                        String::from_utf8_lossy(&output.stderr).trim()
                    ));
                }
                Err(e) => return Err(format!("Signing failed for {}: {e}", path.display())),
            }
        }
    }
    #[cfg(not(windows))]
    let _ = stage;
    Ok(())
}

/// Port of `zip_build`: `dist/WebDeck-<platform>-portable.zip` with a
/// top-level `WebDeck/` folder, matching what the updater extracts.
fn zip_stage(stage: &Path, version: &str) -> Result<PathBuf, String> {
    let _ = version;
    let platform = format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH);
    let suffix = if std::env::args().any(|arg| arg == "--dev") {
        "dev-portable"
    } else {
        "portable"
    };
    let zip_name = format!("WebDeck-{platform}-{suffix}.zip");
    std::fs::create_dir_all("dist").map_err(|e| format!("cannot create dist/: {e}"))?;
    let zip_path = Path::new("dist").join(&zip_name);
    let file = std::fs::File::create(&zip_path).map_err(|e| format!("cannot create zip: {e}"))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut stack = vec![stage.to_path_buf()];
    let mut names: Vec<PathBuf> = Vec::new();
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|e| format!("cannot read stage: {e}"))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("cannot read stage entry: {e}"))?;
            let path = entry.path();
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if kind.is_symlink() {
                return Err("Runtime trees must not contain symlinks".to_string());
            }
            if kind.is_dir() {
                stack.push(path);
            } else if kind.is_file() {
                names.push(path);
            }
        }
    }
    names.sort();
    let root = stage.parent().unwrap_or(Path::new("temp"));
    for path in &names {
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let mut options = options;
        // Preserve executability for Unix binaries (zip has no such concept
        // on Windows; the updater launches `WebDeck.exe` by name there).
        if rel == "WebDeck/WebDeck" || rel == "WebDeck/update" || rel == "WebDeck/webdeck-qr" {
            options = options.unix_permissions(0o755);
        }
        zip.start_file(rel, options)
            .map_err(|e| format!("cannot stage zip entry: {e}"))?;
        let bytes = std::fs::read(path).map_err(|e| format!("cannot read staged file: {e}"))?;
        zip.write_all(&bytes)
            .map_err(|e| format!("cannot write zip entry: {e}"))?;
    }
    zip.finish()
        .map_err(|e| format!("cannot finish zip: {e}"))?;
    Ok(zip_path)
}
