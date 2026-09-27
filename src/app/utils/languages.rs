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
pub fn load_all_lang_files() -> HashMap<String, HashMap<String, String>> {
    let (dir, misc_dir) = state()
        .read()
        .map(|s| (s.dir.clone(), s.misc_dir.clone()))
        .unwrap_or_default();

    let mut files = HashMap::new();
    let mut misc = HashSet::new();

    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.ends_with(".lang") {
                let lang = file_name.trim_end_matches(".lang").to_string();
                if let Ok(dict) = load_lang_file(&lang) {
                    files.insert(lang, dict);
                }
            }
        }
    }
    if !misc_dir.is_empty() {
        if let Ok(entries) = std::fs::read_dir(&misc_dir) {
            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name.ends_with(".lang") {
                    let lang = file_name.trim_end_matches(".lang").to_string();
                    if !files.contains_key(&lang) {
                        if let Ok(dict) = load_lang_file(&lang) {
                            files.insert(lang.clone(), dict);
                        }
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
            code: lang.clone(),
            code_short: data.get("lang_code").cloned().unwrap_or_default(),
            native_name: data.get("native_name").cloned().unwrap_or_default(),
            english_name: data.get("english_name").cloned().unwrap_or_default(),
            author_name: data.get("author_name").cloned().unwrap_or_default(),
            author_github_username: data
                .get("author_github_username")
                .cloned()
                .unwrap_or_default(),
            misc: misc.contains(lang),
        })
        .collect()
}

/// Port of `get_system_language`.
///
/// Python uses `locale.getdefaultlocale()`; Rust std has no locale API, so we
/// parse the `LANG`/`LC_ALL`/`LANGUAGE` environment (e.g. `fr_FR.UTF-8` →
/// `fr_FR`), falling back to the default language.
pub fn get_system_language() -> String {
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
