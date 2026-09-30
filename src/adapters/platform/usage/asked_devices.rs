//! Port of `app/buttons/usage/asked_devices.py`.

use crate::app::utils::settings::get_config::get_config;

/// Port of `extract_asked_device`.
///
/// Same pattern as the `r"\['(.*?)'\]"` regex: every `['...']` group in the
/// message (implemented as a manual scan; no `regex` dependency needed).
pub fn extract_asked_device(input_string: &str) -> Vec<String> {
    let mut matches = Vec::new();
    let bytes = input_string.as_bytes();
    let mut i = 0;
    while i + 4 <= bytes.len() {
        if &bytes[i..i + 2] == b"['" {
            if let Some(end) = input_string[i + 2..].find("']") {
                matches.push(input_string[i + 2..i + 2 + end].to_string());
                i += 2 + end + 2;
                continue;
            }
            break;
        }
        i += 1;
    }
    matches
}

/// Port of `get_asked_devices` — collects `['…']` groups from every
/// `/usage…` button message in the config.
pub fn get_asked_devices() -> Vec<Vec<String>> {
    let mut devices = Vec::new();
    let config = get_config(false, false);
    if let Some(buttons) = config
        .get("front")
        .and_then(|f| f.get("buttons"))
        .and_then(|b| b.as_object())
    {
        for folder_buttons in buttons.values() {
            if let Some(list) = folder_buttons.as_array() {
                for button in list {
                    if let Some(message) = button.get("message").and_then(|m| m.as_str()) {
                        if message.starts_with("/usage") {
                            let device = extract_asked_device(message);
                            if !device.is_empty() {
                                devices.push(device);
                            }
                        }
                    }
                }
            }
        }
    }
    devices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_groups() {
        assert_eq!(
            extract_asked_device("/usage ['cpu'] ['memory','total_gb']"),
            vec!["cpu".to_string(), "memory','total_gb".to_string()]
        );
        assert!(extract_asked_device("/usage").is_empty());
    }
}
