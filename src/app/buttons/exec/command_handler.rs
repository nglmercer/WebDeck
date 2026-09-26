//! Port of `app/buttons/exec/command_handler.py`.
//!
//! Dispatch (`type:uploaded_file` / `type:file_path` / single-line) is ported
//! 1:1, with `threading.Thread(…, daemon=True)` mapped to detached
//! `std::thread::spawn`. Single-line `/exec` Python has no Rust equivalent
//! yet (embedded `rhai` planned); single-line `/batch` already shells out.

use crate::app::buttons::exec::{batch_code, python_code};
use crate::app::utils::logger::log;

fn is_bare_filename(message: &str) -> bool {
    ![":", ".config/user_uploads/", ".config\\user_uploads\\"]
        .iter()
        .any(|substring| message.contains(substring))
}

/// Port of `python`.
pub fn python(message: &str) {
    if message.contains("type:uploaded_file") {
        let message = message
            .replace("C:\\fakepath\\", "")
            .replace("/exec ", "")
            .replace("type:uploaded_file", "");
        let message = message.trim();
        if is_bare_filename(message) {
            // Stored directly in .config/user_uploads, not an absolute path.
            let python_file = format!(".config/user_uploads/{message}");
            log().debug(&format!("Message: {message}, Python file: {python_file}"));
            std::thread::spawn(move || python_code::execute(&python_file));
        }
    } else if message.contains("type:file_path") {
        let python_file = message
            .replace("/exec ", "")
            .replace("type:file_path", "");
        let python_file = python_file.trim().to_string();
        std::thread::spawn(move || python_code::execute(&python_file));
    } else {
        let code = message
            .replace("/exec", "")
            .replace("type:single_line", "");
        // TODO(port): single-line exec via embedded rhai (Python exec()).
        log().warning(&format!(
            "Single-line /exec deferred (rhai planned): {}",
            code.trim()
        ));
    }
}

/// Port of `batch`.
pub fn batch(message: &str) {
    if !cfg!(windows) {
        return;
    }

    if message.contains("type:uploaded_file") {
        let message = message
            .replace("C:\\fakepath\\", "")
            .replace("/batch ", "")
            .replace("type:uploaded_file", "");
        let message = message.trim();
        if is_bare_filename(message) {
            let batch_file = format!(".config/user_uploads/{message}");
            log().debug(&format!("Message: {message}, Batch file: {batch_file}"));
            std::thread::spawn(move || batch_code::execute(&batch_file));
        }
    } else if message.contains("type:file_path") {
        let batch_file = message
            .replace("/batch ", "")
            .replace("type:file_path", "");
        let batch_file = batch_file.trim().to_string();
        std::thread::spawn(move || batch_code::execute(&batch_file));
    } else {
        let command = message.replacen("/batch", "", 1);
        let command = command.trim();
        match std::process::Command::new("cmd").args(["/C", command]).spawn() {
            Ok(_) => {}
            Err(e) => log().debug(&format!("Single-line /batch spawn failed: {e}")),
        }
    }
}
