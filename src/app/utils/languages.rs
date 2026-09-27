//! Port of `app/utils/languages.py`.
//!
//! Same `.lang` file format (`key=value` lines, `//`/`#` comments), same
//! lookup/fallback rules. Module-level Python globals map to a guarded
//! static [`LangState`].
//!
//! One structural deviation: Python stuffs a boolean `misc` flag into each
//! language's string dict; Rust tracks misc languages in a separate set.

use std::collections::{HashMap, HashSet};
use std::sync::{OnceLock, RwLock};

use serde::Serialize;

struct LangState {
    files: HashMap<String, HashMap<String, String>>,
    misc: HashSet<String>,
    default_lang: String,
    dir: String,
    misc_dir: String,
}

impl Default for LangState {
    fn default() -> Self {
        Self {
            files: HashMap::new(),
            misc: HashSet::new(),
            default_lang: "en_US".to_string(),
            dir: String::new(),
            misc_dir: String::new(),
        }
    }
}

static STATE: OnceLock<RwLock<LangState>> = OnceLock::new();

fn state() -> &'static RwLock<LangState> {
    STATE.get_or_init(|| RwLock::new(LangState::default()))
}

/// Resolved translation dict for the TypeScript frontend's `text()`.
pub fn lang_dict(lang: Option<&str>) -> HashMap<String, String> {
    let effective = get_language(lang);
    state()
        .read()
        .ok()
        .and_then(|guard| guard.files.get(&effective).cloned())
        .unwrap_or_default()
}

/// Port of `load_lang_file`.
pub fn load_lang_file(lang: &str) -> Result<HashMap<String, String>, String> {
    let (dir, misc_dir) = state()
        .read()
        .map(|s| (s.dir.clone(), s.misc_dir.clone()))
        .map_err(|e| e.to_string())?;

    let mut lang_path = format!("{dir}/{lang}.lang");
    if !std::path::Path::new(&lang_path).is_file() {
        lang_path = format!("{misc_dir}/{lang}.lang");
    }
    if !std::path::Path::new(&lang_path).is_file() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name.ends_with(".lang") && file_name.starts_with(lang) {
                    lang_path = entry.path().to_string_lossy().to_string();
                    break;
                }
            }
        }
    }

    let content = std::fs::read_to_string(&lang_path)
        .map_err(|e| format!("Cannot read language file '{lang_path}': {e}"))?;
    let mut dictionary = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if !line.is_empty() && !line.starts_with("//") && !line.starts_with('#') {
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| format!("Invalid line format: {line}"))?;
            dictionary.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    Ok(dictionary)
}

/// Port of `get_language`.
pub fn get_language(lang: Option<&str>) -> String {
    let state = state().read();
    let (default_lang, files) = match state.as_ref() {
        Ok(state) => (state.default_lang.clone(), state.files.clone()),
        Err(_) => return "en_US".to_string(),
    };
    drop(state);

    let mut lang = lang.unwrap_or(&default_lang).to_string();
    if lang.eq_ignore_ascii_case("system") {
        lang = get_system_language();
    }
    for available in files.keys() {
        if available.to_lowercase().starts_with(&lang.to_lowercase()) {
            return available.clone();
        }
    }
    default_lang
}

/// Port of `language_exists`.
pub fn language_exists(language_code: &str) -> bool {
    let resolved = get_language(Some(language_code));
    if !resolved.eq_ignore_ascii_case(language_code) {
        return false;
    }
    state()
        .read()
        .map(|s| s.files.contains_key(&resolved))
        .unwrap_or(false)
}

/// Port of `set_default_language`.
pub fn set_default_language(lang: &str) {
    let mut lang = lang.to_string();
    if lang.eq_ignore_ascii_case("system") {
        lang = get_system_language();
    }
    if language_exists(&lang) {
        if let Ok(mut state) = state().write() {
            state.default_lang = lang;
        }
    } else if let Ok(state) = state().read() {
        println!(
            "Language '{lang}' does not exist. Default language remains '{}'.",
            state.default_lang
        );
    }
}

/// Port of `load_all_lang_files`.
/// Language key for a file name — port of Python's `file.split(".")[0]`
/// (so `en_US.extra.lang` keys as `en_US`, quirks included).
fn lang_key(file_name: &str) -> String {
    file_name.split('.').next().unwrap_or("").to_string()
}

pub fn load_all_lang_files() -> HashMap<String, HashMap<String, String>> {
    let (dir, misc_dir) = state()
        .read()
        .map(|s| (s.dir.clone(), s.misc_dir.clone()))
        .unwrap_or_default();

    let mut files = HashMap::new();
    let mut misc = HashSet::new();

    // Python lets every error here propagate (missing directory, unreadable
    // or malformed file) and dies with a traceback during `init`; mirror
    // that instead of skipping bad files.
    let entries =
        std::fs::read_dir(&dir).unwrap_or_else(|_| panic!("No such file or directory: '{dir}'"));
    for entry in entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.ends_with(".lang") {
            let lang = lang_key(&file_name);
            let dict = load_lang_file(&lang)
                .unwrap_or_else(|e| panic!("Error loading language file: {e}"));
            files.insert(lang, dict);
        }
    }
    // The misc directory is optional in Python too (`os.path.isdir` guard).
    if !misc_dir.is_empty() {
        if let Ok(entries) = std::fs::read_dir(&misc_dir) {
            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name.ends_with(".lang") {
                    let lang = lang_key(&file_name);
                    if !files.contains_key(&lang) {
                        let dict = load_lang_file(&lang)
                            .unwrap_or_else(|e| panic!("Error loading language file: {e}"));
                        files.insert(lang.clone(), dict);
                    }
                    misc.insert(lang);
                }
            }
        }
    }

    if let Ok(mut state) = state().write() {
        state.misc = misc;
    }
    files
}

/// Port of `reload_all_lang_files`.
pub fn reload_all_lang_files() {
    let files = load_all_lang_files();
    if let Ok(mut state) = state().write() {
        state.files = files;
    }
}

/// Language metadata — port of the dicts built by `get_languages_info`.
#[derive(Debug, Clone, Serialize)]
pub struct LanguageInfo {
    pub code: String,
    pub code_short: String,
    pub native_name: String,
    pub english_name: String,
    pub author_name: String,
    pub author_github_username: String,
    pub misc: bool,
}

/// Port of `get_languages_info`.
pub fn get_languages_info() -> Vec<LanguageInfo> {
    let snapshot = state().read().map(|s| (s.files.clone(), s.misc.clone()));
    let (files, misc) = match snapshot {
        Ok((files, misc)) if !files.is_empty() => (files, misc),
        _ => {
            let files = load_all_lang_files();
            let misc = state().read().map(|s| s.misc.clone()).unwrap_or_default();
            (files, misc)
        }
    };

    files
        .iter()
        .map(|(lang, data)| LanguageInfo {
            // Python indexes these keys directly (`KeyError` on a corrupt
            // file); direct indexing panics here the same way.
            code: lang.clone(),
            code_short: data["lang_code"].clone(),
            native_name: data["native_name"].clone(),
            english_name: data["english_name"].clone(),
            author_name: data["author_name"].clone(),
            author_github_username: data["author_github_username"].clone(),
            misc: misc.contains(lang),
        })
        .collect()
}

/// Windows system locale name via `GetUserDefaultLocaleName`, normalized to
/// Python's `locale.getdefaultlocale()` format (`en-US` → `en_US`).
#[cfg(windows)]
fn windows_locale_name() -> Option<String> {
    const LOCALE_NAME_MAX_LENGTH: usize = 85;
    let mut buf = [0u16; LOCALE_NAME_MAX_LENGTH];
    // SAFETY: `GetUserDefaultLocaleName` fills at most the slice it is given.
    let written = unsafe { windows::Win32::Globalization::GetUserDefaultLocaleName(&mut buf) };
    if written <= 0 {
        return None;
    }
    let name = String::from_utf16_lossy(&buf[..(written as usize).saturating_sub(1)]);
    if name.is_empty() {
        return None;
    }
    Some(name.replace('-', "_"))
}

/// Port of `get_system_language` (`locale.getdefaultlocale()`).
///
/// On Windows the OS locale API is queried (Python reads it via the C
/// library there); elsewhere the `LC_ALL`/`LANG`/`LANGUAGE` environment is
/// parsed, which is what `getdefaultlocale()` itself consults. Falls back
/// to the default language.
pub fn get_system_language() -> String {
    #[cfg(windows)]
    {
        if let Some(code) = windows_locale_name() {
            return code;
        }
    }
    for var in ["LC_ALL", "LANG", "LANGUAGE"] {
        if let Ok(value) = std::env::var(var) {
            let code = value
                .split('.')
                .next()
                .unwrap_or("")
                .split(':')
                .next()
                .unwrap_or("")
                .to_string();
            if !code.is_empty() && code != "C" && code != "POSIX" {
                return code;
            }
        }
    }
    state()
        .read()
        .map(|s| s.default_lang.clone())
        .unwrap_or_else(|_| "en_US".to_string())
}

/// Port of `init`.
pub fn init(
    lang_files_directory: &str,
    misc_lang_files_directory: Option<&str>,
    default_language: &str,
) -> Vec<LanguageInfo> {
    if lang_files_directory.is_empty() {
        panic!("'lang_files_directory' must be specified");
    }
    if let Ok(mut state) = state().write() {
        if let Some(misc) = misc_lang_files_directory {
            state.misc_dir = misc.to_string();
        }
        state.dir = lang_files_directory.to_string();
    }
    reload_all_lang_files();
    set_default_language(default_language);
    get_languages_info()
}

/// Port of `text`.
pub fn text(key: Option<&str>, lang: Option<&str>) -> String {
    let Some(key) = key else {
        return String::new();
    };
    // Requested → resolved → default fallback chain (mirrors text() in
    // app/utils/languages.py).
    let requested = lang
        .map(|s| s.to_string())
        .or_else(|| state().read().ok().map(|s| s.default_lang.clone()))
        .unwrap_or_else(|| "en_US".to_string());
    let resolved = get_language(Some(&requested));
    let (files, default_lang) = match state().read().as_ref() {
        Ok(state) => (state.files.clone(), state.default_lang.clone()),
        Err(_) => return key.to_string(),
    };

    let effective = if files.contains_key(&resolved) {
        resolved
    } else if files.contains_key(&default_lang) {
        default_lang.clone()
    } else {
        panic!("Language '{default_lang}' not found in lang_files");
    };
    files
        .get(&effective)
        .and_then(|dict| dict.get(key))
        .cloned()
        .unwrap_or_else(|| key.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_real_lang_files() {
        // Runs from the package root, where webdeck/translations exists.
        init(
            "webdeck/translations",
            Some("webdeck/translations/misc"),
            "en_US",
        );
        assert!(language_exists("en_US"));
        let missing = text(Some("definitely_not_a_key_zzz"), None);
        assert_eq!(missing, "definitely_not_a_key_zzz");
        let infos = get_languages_info();
        assert!(infos.iter().any(|info| info.code == "en_US"));
    }
}
