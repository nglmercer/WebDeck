//! Port of `app/utils/args.py`.
//!
//! Same flags, same `--help` text intent, same startup side effects
//! ([`handle_startup_arguments`]).
//!
//! Mapping notes vs Python:
//! - `argparse` → `clap` (`Args` struct). `get_arg('port')` becomes
//!   `get_args().port`, `get_arg('no_tray')` becomes `get_args().no_tray`, etc.
//! - Python persists parsed args to `temp/webdeck_args.json` and deletes it on
//!   load; that file is only ever read back in-process, so Rust keeps the
//!   parsed `Args` in a static instead. [`save_args`]/[`load_args`]/
//!   [`clear_args`] keep their names as thin shims over that static.
//! - Like Python, `raw_args()` returns the *sorted*, filtered argv (upstream
//!   quirk preserved 1:1).

use std::sync::{Mutex, OnceLock};

use clap::Parser;

use crate::app::buttons::soundboard::ffmpeg;
#[cfg(windows)]
use crate::app::utils::show_error::show_error;
use crate::app::utils::{exit::exit_program, languages, logger::log};

/// Exit-command aliases — port of the `positionals["exit"]["choices"]` list.
pub const EXIT_CHOICES: &[&str] = &[
    "exit",
    "stop",
    "close",
    "quit",
    "kill",
    "terminate",
    "shutdown",
];

/// CLI flags — port of `available_args` + `positionals` in `app/utils/args.py`.
#[derive(Parser, Debug, Clone)]
#[command(name = "WebDeck", disable_version_flag = true)]
pub struct Args {
    /// Display the application version and exit
    #[arg(short = 'v', long = "version")]
    pub version: bool,

    /// Specify a custom port for the server
    #[arg(short = 'p', long = "port")]
    pub port: Option<u16>,

    /// Specify the host for the server
    #[arg(short = 'H', long = "host")]
    pub host: Option<String>,

    /// Specify the timeout in seconds to close the app after
    #[arg(short = 't', long = "timeout")]
    pub timeout: Option<u64>,

    /// Run the application without requesting sudo/admin permissions
    #[arg(long = "no-admin", alias = "no-sudo")]
    pub no_admin: bool,

    /// Run the application without the system tray icon
    #[arg(long = "no-tray")]
    pub no_tray: bool,

    /// Disable the printing of debug messages (does not affect server debug mode)
    #[arg(long = "no-debug")]
    pub no_debug: bool,

    /// Authorize the start of the app even if it is already running
    #[arg(long = "force-start")]
    pub force_start: bool,

    /// Specify a custom log file path
    #[arg(long = "log-file")]
    pub log_file: Option<String>,

    /// Force update the application to the latest version
    #[arg(long = "force-update", alias = "update")]
    pub force_update: bool,

    /// Disable the auto-update feature
    #[arg(long = "no-auto-update", alias = "no-update")]
    pub no_auto_update: bool,

    /// [DEV] Simulate an error to test the error handling
    #[cfg_attr(not(debug_assertions), arg(hide = true))]
    #[arg(long = "fake-error")]
    pub fake_error: bool,

    /// [DEV] Attempt to install FFmpeg if it is not found
    #[cfg_attr(not(debug_assertions), arg(hide = true))]
    #[arg(long = "test-ffmpeg")]
    pub test_ffmpeg: bool,

    /// Exit all instances of WebDeck running on the device
    pub exit: Option<String>,
}

impl Default for Args {
    /// Defaults matching `parser.parse_args([])` in `app/utils/args.py`.
    fn default() -> Self {
        Self {
            version: false,
            port: None,
            host: None,
            timeout: None,
            no_admin: false,
            no_tray: false,
            no_debug: false,
            force_start: false,
            log_file: None,
            force_update: false,
            no_auto_update: false,
            fake_error: false,
            test_ffmpeg: false,
            exit: None,
        }
    }
}

static ARGS: OnceLock<Mutex<Option<Args>>> = OnceLock::new();

fn slot() -> &'static Mutex<Option<Args>> {
    ARGS.get_or_init(|| Mutex::new(None))
}

/// Port of the module-level `raw_args` list in `app/utils/args.py`.
pub fn raw_args() -> Vec<String> {
    let mut args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| !(arg.ends_with(".pyc") || arg.ends_with("library.zip")))
        .collect();
    args.sort();
    args
}

/// Access the parsed CLI args (port of the `get_arg('…')` call sites).
pub fn get_args() -> Args {
    load_args()
}

/// Port of `parse_args`.
pub fn parse_args() {
    clear_args();

    let argv: Vec<String> = std::iter::once(
        std::env::args()
            .next()
            .unwrap_or_else(|| "webdeck".to_string()),
    )
    .chain(std::env::args().skip(1))
    .collect();

    match Args::try_parse_from(&argv) {
        Ok(parsed) => {
            reject_dev_flags_in_release(&parsed);
            if !parsed.no_debug && !parsed.version {
                log().debug(&format!("All args: {parsed:?}"));
            }
            save_args(parsed);
        }
        // argparse calls `sys.exit` on any parse error (`SystemExit` is not
        // caught by Python's `except Exception`), so a bad invocation exits
        // instead of starting with defaults. `e.exit()` reproduces that:
        // usage error + exit code 2 (exit code 0 for `--help`).
        Err(e) => e.exit(),
    }

    handle_startup_arguments();
}

/// Frozen Python does not register `--fake-error` / `--test-ffmpeg`
/// (`"condition": not frozen`), so passing them is an unrecognized-argument
/// error there. In release builds (`cfg!(debug_assertions)` ⇔ unfrozen)
/// reject them the same way instead of running the dev action.
fn reject_dev_flags_in_release(parsed: &Args) {
    #[cfg(not(debug_assertions))]
    {
        use clap::CommandFactory;
        let flag = if parsed.fake_error {
            Some("--fake-error")
        } else if parsed.test_ffmpeg {
            Some("--test-ffmpeg")
        } else {
            None
        };
        if let Some(flag) = flag {
            Args::command()
                .error(
                    clap::error::ErrorKind::UnknownArgument,
                    format!("unexpected argument '{flag}' found"),
                )
                .exit();
        }
    }
    #[cfg(debug_assertions)]
    let _ = parsed;
}

/// Port of `clear_args`.
pub fn clear_args() {
    if let Ok(mut slot) = slot().lock() {
        *slot = None;
    }
    // Also drop a stale temp file left by the Python app, if any.
    let _ = std::fs::remove_file("temp/webdeck_args.json");
}

/// Port of `save_args`.
pub fn save_args(args: Args) {
    if let Ok(mut slot) = slot().lock() {
        *slot = Some(args);
    }
}

/// Port of `load_args`.
pub fn load_args() -> Args {
    slot()
        .lock()
        .ok()
        .and_then(|slot| slot.clone())
        .unwrap_or_default()
}

/// Port of `handle_startup_arguments`.
pub fn handle_startup_arguments() {
    let args = load_args();

    // --no-debug
    if args.no_debug {
        log().disable_debug();
    }

    // -v, --version
    if args.version {
        let version = std::fs::read_to_string("webdeck/version.json")
            .ok()
            .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
            .and_then(|v| {
                v.get("versions")?
                    .get(0)?
                    .get("version")?
                    .as_str()
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "unknown".to_string());

        let mut last_commit = String::new();
        if cfg!(debug_assertions) && std::path::Path::new(".git").is_dir() {
            if let Ok(output) = std::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .output()
            {
                if output.status.success() {
                    let sha = String::from_utf8_lossy(&output.stdout);
                    let short: String = sha.trim().chars().take(7).collect();
                    last_commit = format!("({short})");
                }
            }
        }

        println!("WebDeck v{version} {last_commit}");
        std::process::exit(0);
    }

    // -t TIMEOUT, --timeout TIMEOUT (Python uses threading.Timer; same here).
    if let Some(timeout) = args.timeout {
        log().info(&format!("Setting timeout to {timeout} seconds"));
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(timeout));
            exit_program(true, true);
        });
    }

    // --log-file
    if let Some(log_file) = args.log_file.as_deref() {
        log().set_log_file(std::path::Path::new(log_file));
        log().info(&format!("Logging to file: {log_file}"));
    }

    // --fake-error
    if args.fake_error {
        let result: Result<(), _> = Err("division by zero".to_string());
        if let Err(e) = result {
            languages::init(
                "webdeck/translations",
                Some("webdeck/translations/misc"),
                "en_US",
            );
            #[cfg(windows)]
            show_error(
                None,
                "WebDeck Error",
                true,
                Some(&e as &dyn std::fmt::Debug),
            );
            #[cfg(not(windows))]
            log().exception(
                &e,
                Some("Failed to initialize tray icon"),
                false,
                true,
                true,
            );
        }
    }

    // --test-ffmpeg
    if args.test_ffmpeg {
        log().debug("Testing FFmpeg installation...");
        let path = ffmpeg::get_ffmpeg();
        log().debug(&format!("FFmpeg path: {path:?}"));
        exit_program(true, false);
    }

    // exit
    if let Some(exit_cmd) = args.exit.as_deref() {
        if EXIT_CHOICES.contains(&exit_cmd) {
            exit_program(true, false);
        } else {
            log().error("Invalid command provided.");
            exit_program(true, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_python() {
        let args = Args::default();
        assert!(!args.version && args.port.is_none() && args.exit.is_none());
        assert!(!args.no_admin && !args.force_update);
    }

    #[test]
    fn parses_flags() {
        let args = Args::try_parse_from(["webdeck", "-p", "8080", "--no-tray", "--no-sudo"])
            .expect("should parse");
        assert_eq!(args.port, Some(8080));
        assert!(args.no_tray);
        assert!(args.no_admin);
    }
}
