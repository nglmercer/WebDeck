//! Port of `app/buttons/color_picker/get_arg.py`.

/// Port of `getarg` — returns the value of the `arg:` token in the message.
pub fn getarg(message: &str, arg: &str) -> Option<String> {
    let prefix = format!("{arg}:");
    message
        .split_whitespace()
        .find(|token| token.starts_with(&prefix))
        .map(|token| token.split_at(prefix.len()).1.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_args() {
        let msg = "/colorpicker lang:en type:text;hex copy:hex";
        assert_eq!(getarg(msg, "lang"), Some("en".to_string()));
        assert_eq!(getarg(msg, "type"), Some("text;hex".to_string()));
        assert_eq!(getarg(msg, "missing"), None);
    }
}
