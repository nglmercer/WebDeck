//! Port of `app/utils/translate.py`.

use crate::app::utils::logger::log;

/// Port of `translate`.
///
/// The camelCase spacing and the EN fast path are ported 1:1. The
/// `deep_translator` network call is TODO (planned: `reqwest` against the
/// same Google Translate web endpoint); until then non-English targets get
/// the spaced English word back, matching the Python `NameError` fallback.
pub fn translate(word: &str, target_language: &str) -> String {
    // Separate words with spaces before each capital letter.
    let mut spaced = String::with_capacity(word.len() + 4);
    for ch in word.chars() {
        if ch.is_uppercase() {
            spaced.push(' ');
        }
        spaced.push(ch);
    }
    let spaced = spaced.trim().to_string();

    if target_language.eq_ignore_ascii_case("EN") {
        return spaced;
    }

    // TODO(port): online translation via reqwest (deep_translator equivalent).
    log().debug(&format!(
        "translate({word:?}, {target_language:?}): online translation not ported yet, returning English"
    ));
    spaced
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_camel_case() {
        assert_eq!(translate("helloWorld", "EN"), "hello World");
        assert_eq!(translate("already spaced", "EN"), "already spaced");
    }
}
