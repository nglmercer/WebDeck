//! Port of `app/utils/translate.py`.
//!
//! `deep_translator` is a thin wrapper over Google Translate's free web
//! endpoint; this port calls the same `translate.googleapis.com` endpoint
//! directly with `reqwest` (blocking — callers run off the async runtime).

use crate::app::utils::logger::log;

/// Split `camelCase` into spaced words (ported helper, also unit-tested).
pub fn space_camel_case(word: &str) -> String {
    let mut spaced = String::with_capacity(word.len() + 4);
    for ch in word.chars() {
        if ch.is_uppercase() {
            spaced.push(' ');
        }
        spaced.push(ch);
    }
    spaced.trim().to_string()
}

/// Parse the `translate_a/single` response: `[[["translated",...],...],...]`.
/// Returns the concatenated first elements, or `None` on unexpected shapes.
pub fn parse_translate_response(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let segments = value.get(0)?.as_array()?;
    let mut out = String::new();
    for segment in segments {
        if let Some(text) = segment.get(0).and_then(|v| v.as_str()) {
            out.push_str(text);
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn translate_online(word: &str, target_language: &str) -> Option<String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?;
    let response = client
        .get("https://translate.googleapis.com/translate_a/single")
        .query(&[
            ("client", "gtx"),
            ("sl", "en"),
            ("tl", target_language),
            ("dt", "t"),
            ("q", word),
        ])
        .send()
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let body = response.text().ok()?;
    parse_translate_response(&body)
}

/// Port of `translate`.
pub fn translate(word: &str, target_language: &str) -> String {
    // Separate words with spaces before each capital letter.
    let spaced = space_camel_case(word);

    if target_language.eq_ignore_ascii_case("EN") {
        return spaced;
    }

    match translate_online(&spaced, &target_language.to_lowercase()) {
        Some(translated) => translated,
        None => {
            // Mirrors the Python `NameError` fallback (return English).
            log().debug(&format!(
                "translate({word:?}, {target_language:?}): online translation failed, returning English"
            ));
            spaced
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_camel_case() {
        assert_eq!(space_camel_case("helloWorld"), "hello World");
        assert_eq!(translate("helloWorld", "EN"), "hello World");
        assert_eq!(translate("already spaced", "EN"), "already spaced");
    }

    #[test]
    fn parses_google_response() {
        let body = r#"[[["Hola Mundo","hello world",null,null,1]],null,"en"]"#;
        assert_eq!(
            parse_translate_response(body),
            Some("Hola Mundo".to_string())
        );
        assert_eq!(parse_translate_response("not json"), None);
        assert_eq!(parse_translate_response("{}"), None);
    }

    #[test]
    #[ignore = "requires network access to translate.googleapis.com"]
    fn live_translation() {
        let translated = translate("helloWorld", "es");
        assert!(!translated.is_empty());
        assert_ne!(translated, "hello World");
    }
}
