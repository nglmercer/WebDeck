//! Port of `app/buttons/soundboard/ffmpeg.py`.
//!
//! Install/discovery logic is 1:1: cached path → `./ffmpeg.exe` → WinGet
//! package search → WebDeck-servers download → `winget install`. The `pydub`
//! ops (`add_silence_to_end`, `to_wav`) invoke the discovered `ffmpeg`
//! binary directly with equivalent filters (`apad`, `volume`), since pydub
//! has no Rust equivalent.
//!
//! Documented deviations (all graceful where Python crashes):
//! - The WinGet search continues past unreadable directories instead of
//!   aborting the whole search on the first missing one (Python bug).
//! - A failed `winget install` returns `None` instead of recursing forever.
//! - Missing `ffprobe.exe` / missing inputs fail gracefully instead of
//!   raising out of `get_ffmpeg` / crashing on an undefined `audio`.

use std::path::Path;
use std::sync::{Mutex, OnceLock};

use crate::app::utils::logger::log;

/// Port of the `is_downloading` / `ffmpeg_path` module globals.
struct FfmpegState {
    is_downloading: bool,
    ffmpeg_path: String,
}

fn state() -> &'static Mutex<FfmpegState> {
    static STATE: OnceLock<Mutex<FfmpegState>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(FfmpegState {
            is_downloading: false,
            ffmpeg_path: String::new(),
        })
    })
}

fn abspath(path: &str) -> String {
    if let Ok(abs) = std::path::absolute(path) {
        return abs.to_string_lossy().into_owned();
    }
    std::env::current_dir()
        .map(|cwd| cwd.join(path).to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string())
}

/// Platform ffmpeg binary name (`ffmpeg.exe` on Windows, `ffmpeg` else).
#[cfg(windows)]
const FFMPEG_EXE: &str = "ffmpeg.exe";
/// Platform ffmpeg binary name (`ffmpeg.exe` on Windows, `ffmpeg` else).
#[cfg(not(windows))]
const FFMPEG_EXE: &str = "ffmpeg";

/// Find a tool on PATH (no extra dependency for a three-line search).
#[cfg(target_os = "linux")]
fn find_on_path(tool: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(tool))
        .find(|candidate| candidate.is_file())
        .map(|candidate| candidate.to_string_lossy().into_owned())
}

/// Port of `install_ffmpeg`.
pub fn install_ffmpeg() -> Option<String> {
    // Check if ffmpeg is already installed in the system (cached).
    if let Ok(st) = state().lock() {
        if !st.ffmpeg_path.is_empty() && Path::new(&st.ffmpeg_path).is_file() {
            return Some(st.ffmpeg_path.clone());
        }
    }

    // Check if ffmpeg is already installed in the current directory.
    if Path::new(FFMPEG_EXE).is_file() {
        return Some(abspath(FFMPEG_EXE));
    }

    // Linux: system ffmpeg via PATH (distro packages). No download: the
    // WebDeck-served zip and winget are Windows-only.
    #[cfg(target_os = "linux")]
    {
        if let Some(path) = find_on_path("ffmpeg") {
            if let Ok(mut st) = state().lock() {
                st.ffmpeg_path = path.clone();
            }
            log().debug(&format!("ffmpeg path found: {path}"));
            return Some(path);
        }
        log().error(
            "FFMPEG: not found. Install it with your package manager (e.g. `pacman -S ffmpeg` / `apt install ffmpeg`).",
        );
        return None;
    }

    #[cfg(windows)]
    {
        install_ffmpeg_windows()
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    {
        log().error("FFMPEG: not found.");
        None
    }
}

/// Windows half of [`install_ffmpeg`]: WinGet search → WebDeck-servers
/// download → `winget install`.
#[cfg(windows)]
fn install_ffmpeg_windows() -> Option<String> {
    // Search for ffmpeg installation on winget.
    if let Some(found) = search_winget_ffmpeg() {
        if let Ok(mut st) = state().lock() {
            st.ffmpeg_path = found.clone();
        }
        log().debug(&format!("ffmpeg path found: {found}"));
        return Some(found);
    }

    // Install ffmpeg via webdeck servers.
    let should_download = match state().lock() {
        Ok(mut st) => {
            if !st.is_downloading {
                st.is_downloading = true;
                true
            } else {
                false
            }
        }
        Err(_) => false,
    };
    if should_download {
        match download_ffmpeg() {
            Ok(path) => {
                if let Ok(mut st) = state().lock() {
                    st.is_downloading = false;
                }
                return Some(path);
            }
            Err(e) => {
                // NOTE: Python leaves `is_downloading = True` here, which also
                // skips the winget fallback below; mirrored 1:1.
                log().exception(
                    &e,
                    Some("FFMPEG: Error occurred while downloading ffmpeg using webdeck servers"),
                    true,
                    true,
                    true,
                );
            }
        }
    }

    // Install ffmpeg via winget.
    let should_winget = state().lock().map(|st| !st.is_downloading).unwrap_or(false);
    if should_winget {
        log().info("FFMPEG: downloading ffmpeg using winget...");
        let result = std::process::Command::new("winget")
            .args(["install", "ffmpeg"])
            .status();
        if let Ok(mut st) = state().lock() {
            st.is_downloading = false;
        }
        match result {
            Ok(status) if status.success() => return install_ffmpeg(),
            Ok(status) => {
                // Python recurses unconditionally here (infinite loop when the
                // install keeps failing); stop instead.
                log().exception(
                    &format!("winget exited with {status}"),
                    Some("FFMPEG: Error occurred while downloading ffmpeg using winget"),
                    true,
                    true,
                    true,
                );
            }
            Err(e) => {
                log().exception(
                    &e,
                    Some("FFMPEG: Error occurred while downloading ffmpeg using winget"),
                    true,
                    true,
                    true,
                );
            }
        }
    }

    log().error("FFMPEG: not found.");
    None
}

/// WinGet package search (`C:/Users/*/AppData/.../Gyan.FFmpeg*/ffmpeg-*/bin/ffmpeg.exe`).
/// Unlike Python, unreadable directories are skipped instead of aborting the
/// whole search, and missing roots stay silent (they never exist off Windows).
#[cfg(windows)]
fn search_winget_ffmpeg() -> Option<String> {
    let users = std::fs::read_dir("C:/Users").ok()?;
    for user_dir in users.flatten() {
        if !user_dir.file_type().ok()?.is_dir() {
            continue;
        }
        let packages = user_dir
            .path()
            .join("AppData/Local/Microsoft/WinGet/Packages");
        for package_dir in std::fs::read_dir(packages).ok()?.flatten() {
            if !package_dir
                .file_name()
                .to_string_lossy()
                .starts_with("Gyan.FFmpeg")
            {
                continue;
            }
            for sub in std::fs::read_dir(package_dir.path()).ok()?.flatten() {
                if sub.file_type().ok()?.is_dir()
                    && sub.file_name().to_string_lossy().starts_with("ffmpeg-")
                {
                    let bin = sub.path().join("bin/ffmpeg.exe");
                    if bin.exists() {
                        return Some(bin.to_string_lossy().into_owned());
                    }
                }
            }
        }
    }
    None
}

/// Download the WebDeck-served ffmpeg zip, extract it in place, delete the zip.
#[cfg(windows)]
fn download_ffmpeg() -> Result<String, String> {
    log().info("FFMPEG: downloading ffmpeg using webdeck servers...");
    let url = "https://bishokus.fr/dl_ffmpeg";
    let zip_path = "ffmpeg-N-114554-g7bf85d2d3a-win64-gpl.zip";
    let bytes = reqwest::blocking::get(url)
        .map_err(|e| e.to_string())?
        .bytes()
        .map_err(|e| e.to_string())?;
    std::fs::write(zip_path, &bytes).map_err(|e| e.to_string())?;
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    archive.extract("").map_err(|e| e.to_string())?;
    std::fs::remove_file(zip_path).map_err(|e| e.to_string())?;
    Ok(abspath("ffmpeg.exe"))
}

/// Port of `get_ffmpeg`: ensure ffmpeg exists, stage `ffmpeg.exe` /
/// `ffprobe.exe` next to the app, return the staged binary path.
pub fn get_ffmpeg() -> Option<String> {
    let ffmpeg_path = install_ffmpeg()?;

    // Linux: system installs are on PATH; use the resolved binary directly.
    #[cfg(not(windows))]
    return Some(ffmpeg_path);

    // Windows: stage ffmpeg.exe / ffprobe.exe in CWD like Python.
    #[cfg(windows)]
    {
        if abspath(&ffmpeg_path) != abspath("ffmpeg.exe") {
            let _ = std::fs::copy(&ffmpeg_path, "ffmpeg.exe");
        }
        let probe_src = ffmpeg_path.replace("ffmpeg.exe", "ffprobe.exe");
        if abspath(&probe_src) != abspath("ffprobe.exe") {
            // Python raises here when ffprobe is missing; ignore instead.
            let _ = std::fs::copy(&probe_src, "ffprobe.exe");
        }

        Some(abspath("ffmpeg.exe"))
    }
}

/// Port of `replace_last_element`.
pub fn replace_last_element(string: &str, old_element: &str, new_element: &str) -> String {
    match string.rfind(old_element) {
        Some(last_index) => {
            format!(
                "{}{}{}",
                &string[..last_index],
                &string[last_index..].replacen(old_element, new_element, 1),
                ""
            )
        }
        None => string.to_string(),
    }
}

/// Port of `add_silence_to_end` via `ffmpeg -af apad=pad_dur=…`.
pub fn add_silence_to_end(input_file: &str, output_file: &str, silence_duration_ms: u64) -> bool {
    let abs_in = abspath(input_file);
    if !Path::new(&abs_in).is_file() {
        log().exception(
            &format!("input not found: {abs_in}"),
            Some("Error occurred while loading the audio file"),
            true,
            true,
            true,
        );
        if get_ffmpeg().is_none() {
            return false;
        }
        // Python proceeds here and crashes on the undefined `audio`;
        // fail gracefully instead.
        if !Path::new(&abs_in).is_file() {
            return false;
        }
    }
    let Some(bin) = get_ffmpeg() else {
        return false;
    };
    let pad = format!("apad=pad_dur={:.3}", silence_duration_ms as f64 / 1000.0);
    match std::process::Command::new(&bin)
        .args(["-y", "-i", &abs_in, "-af", &pad, output_file])
        .status()
    {
        Ok(status) if status.success() => true,
        Ok(status) => {
            log().exception(
                &format!("ffmpeg exited with {status}"),
                Some("Error occurred while adding silence to the audio file"),
                true,
                true,
                true,
            );
            false
        }
        Err(e) => {
            log().exception(
                &e,
                Some("Error occurred while adding silence to the audio file"),
                true,
                true,
                true,
            );
            false
        }
    }
}

/// Port of `silence_path`: cached `_.mp3` with 2ms of trailing silence.
/// Returns `None` where Python returns `False`.
pub fn silence_path(input_file: &str, remove_previous: bool) -> Option<String> {
    let output_file = replace_last_element(input_file, ".mp3", "_.mp3");
    if Path::new(&output_file).exists() {
        return Some(input_file.to_string());
    }
    if !add_silence_to_end(input_file, &output_file, 2) {
        return None;
    }
    if remove_previous {
        let _ = std::fs::remove_file(input_file);
    }
    Some(output_file)
}

/// Port of `to_wav` via `ffmpeg -af volume=…dB`. Returns `None` where
/// Python returns `False` (or crashes when ffmpeg is missing).
pub fn to_wav(input_file: &str, output_file: Option<&str>, volume: f32) -> Option<String> {
    let bin = get_ffmpeg();

    // Set default output file name if not provided.
    let output_file = output_file.map(str::to_string).unwrap_or_else(|| {
        replace_last_element(
            input_file,
            ".mp3",
            &format!("_vol{}.wav", (volume * 100.0) as i32),
        )
    });

    // Check if the output file already exists.
    if Path::new(&output_file).exists() {
        return Some(output_file);
    }

    let abs_in = abspath(input_file);
    if !Path::new(&abs_in).is_file() {
        log().exception(
            &format!("input not found: {abs_in}"),
            Some("Error occurred while loading the audio file"),
            true,
            true,
            true,
        );
        return None;
    }
    let Some(bin) = bin else {
        return None;
    };

    // pydub `audio + volume * 10` dB, exported as WAV.
    let gain = format!("volume={:.1}dB", volume * 10.0);
    match std::process::Command::new(&bin)
        .args(["-y", "-i", &abs_in, "-af", &gain, &output_file])
        .status()
    {
        Ok(status) if status.success() => Some(output_file),
        Ok(status) => {
            log().exception(
                &format!("ffmpeg exited with {status}"),
                Some("Error occurred while converting the audio file to wav"),
                true,
                true,
                true,
            );
            None
        }
        Err(e) => {
            log().exception(
                &e,
                Some("Error occurred while converting the audio file to wav"),
                true,
                true,
                true,
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_only_last_occurrence() {
        assert_eq!(replace_last_element("a-b-a", "a", "z"), "a-b-z");
        assert_eq!(replace_last_element("aaa", "a", "b"), "aab");
        assert_eq!(replace_last_element("abc", "z", "q"), "abc");
    }

    #[test]
    fn silence_output_name_matches_python() {
        assert_eq!(replace_last_element("x.mp3", ".mp3", "_.mp3"), "x_.mp3");
        assert_eq!(
            replace_last_element("a.mp3.mp3", ".mp3", "_.mp3"),
            "a.mp3_.mp3"
        );
    }
}
