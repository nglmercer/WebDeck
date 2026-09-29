//! Port of `app/utils/logger.py`.
//!
//! Same API and same log-file layout (`.logs/<date>.log`, `.logs/<date>-updater.log`).
//! Color output uses inline ANSI codes (no `colorama` equivalent needed).
//!
//! The module-level `log = Logger()` singleton maps to [`log()`]; the updater's
//! `Logger(from_updater=True)` maps to [`updater_log()`].

use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex, OnceLock,
};

// ANSI colors mirroring colorama Fore.* usage in logger.py.
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const MAGENTA: &str = "\x1b[35m";
const CYAN: &str = "\x1b[36m";
const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";

fn days_to_civil_date(days_since_epoch: i64) -> (i32, u32, u32) {
    // Howard Hinnant's civil_from_days algorithm; avoids a chrono dependency.
    let z = days_since_epoch + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = (if m <= 2 { y + 1 } else { y }) as i32;
    (y, m, d)
}

/// Local date/time parts — Python's `datetime.now()` is local, not UTC.
fn now_local_parts() -> Option<(i32, u32, u32, u32, u32, u32)> {
    #[cfg(unix)]
    {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as libc::time_t)
            .unwrap_or(0);
        let mut broken = std::mem::MaybeUninit::<libc::tm>::zeroed();
        // SAFETY: `localtime_r` writes a valid `struct tm` into `broken`
        // (or returns null on failure, which we check).
        let ok = unsafe { !libc::localtime_r(&secs, broken.as_mut_ptr()).is_null() };
        if !ok {
            return None;
        }
        let broken = unsafe { broken.assume_init() };
        Some((
            broken.tm_year + 1900,
            (broken.tm_mon + 1) as u32,
            broken.tm_mday as u32,
            broken.tm_hour as u32,
            broken.tm_min as u32,
            broken.tm_sec as u32,
        ))
    }
    #[cfg(windows)]
    {
        // SAFETY: `GetLocalTime` takes no pointers in windows 0.62.
        let systime = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
        return Some((
            systime.wYear as i32,
            systime.wMonth as u32,
            systime.wDay as u32,
            systime.wHour as u32,
            systime.wMinute as u32,
            systime.wSecond as u32,
        ));
    }
    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}

fn now_date_and_time() -> (String, String) {
    if let Some((y, m, d, hh, mm, ss)) = now_local_parts() {
        (
            format!("{y:04}-{m:02}-{d:02}"),
            format!("{hh:02}:{mm:02}:{ss:02}"),
        )
    } else {
        // Fallback: UTC via the civil-date algorithm (avoids a chrono dep).
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let (y, m, d) = days_to_civil_date(secs.div_euclid(86_400));
        let day_secs = secs.rem_euclid(86_400);
        let (hh, mm, ss) = (day_secs / 3600, (day_secs % 3600) / 60, day_secs % 60);
        (
            format!("{y:04}-{m:02}-{d:02}"),
            format!("{hh:02}:{mm:02}:{ss:02}"),
        )
    }
}

fn default_log_file(from_updater: bool) -> PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    dir.push(".logs");
    let _ = std::fs::create_dir_all(&dir);
    let (date, _) = now_date_and_time();
    let name = if from_updater {
        format!("{date}-updater.log")
    } else {
        format!("{date}.log")
    };
    dir.join(name)
}

/// Port of the `Logger` class in `app/utils/logger.py`.
pub struct Logger {
    log_file: Mutex<PathBuf>,
    debug_enabled: AtomicBool,
}

impl Logger {
    pub fn new(from_updater: bool) -> Self {
        Self {
            log_file: Mutex::new(default_log_file(from_updater)),
            debug_enabled: AtomicBool::new(true),
        }
    }

    /// Port of `Logger._write_log`.
    fn write_log(&self, level: &str, message: &str) -> String {
        let (_, timestamp) = now_date_and_time();
        let log_message = format!("[{level} @ {timestamp}] - {message}");
        if let Ok(path) = self.log_file.lock() {
            use std::io::Write;
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path.as_path())
            {
                let _ = writeln!(file, "{log_message}");
            }
        }
        log_message
    }

    /// Port of `Logger.set_log_file`.
    pub fn set_log_file(&self, log_file: &Path) {
        if let Ok(mut path) = self.log_file.lock() {
            *path = log_file.to_path_buf();
        }
    }

    /// Port of `Logger.enable_debug`.
    pub fn enable_debug(&self) {
        self.debug_enabled.store(true, Ordering::SeqCst);
    }

    /// Port of `Logger.disable_debug`.
    pub fn disable_debug(&self) {
        self.debug_enabled.store(false, Ordering::SeqCst);
    }

    /// Port of `Logger.info`.
    pub fn info(&self, message: &str) {
        let log_message = self.write_log("INFO", message);
        println!("{GREEN}{log_message}{RESET}");
    }

    /// Port of `Logger.success`.
    pub fn success(&self, message: &str) {
        let log_message = self.write_log("SUCCESS", message);
        println!("{GREEN}{log_message}{RESET}");
    }

    /// Port of `Logger.notice`.
    pub fn notice(&self, message: &str) {
        let log_message = self.write_log("NOTICE", message);
        println!("{GREEN}{log_message}{RESET}");
    }

    /// Port of `Logger.warning`.
    pub fn warning(&self, message: &str) {
        let log_message = self.write_log("WARNING", message);
        println!("{YELLOW}{log_message}{RESET}");
    }

    /// Port of `Logger.debug`.
    pub fn debug(&self, message: &str) {
        let log_message = self.write_log("DEBUG", message);
        if self.debug_enabled.load(Ordering::SeqCst) {
            println!("{BLUE}{log_message}{RESET}");
        }
    }

    /// Port of `Logger.error`.
    pub fn error(&self, message: &str) {
        let log_message = self.write_log("ERROR", message);
        println!("{RED}{log_message}{RESET}");
    }

    /// Port of `Logger.exception`.
    ///
    /// Rust has no chained tracebacks by default; the `Debug` rendering of the
    /// error plays the role of `traceback.format_exception`. `log_traceback`
    /// toggles between `Debug` (`{:?}`) and `Display`-style (`{}`) rendering
    /// for errors implementing both; non-`Display` errors always use `Debug`.
    pub fn exception<E: Debug + ?Sized>(
        &self,
        error: &E,
        message: Option<&str>,
        expected: bool,
        log_traceback: bool,
        print_log: bool,
    ) {
        let exception_title = format!("{:?}:\n {error:?}\n", std::any::type_name::<E>());
        let exception_message = if log_traceback {
            // Closest port of Python's `traceback` attachment: Rust errors
            // carry no raise-site stack, so capture the log-site backtrace.
            format!(
                "{error:#?}\nStack:\n{:?}",
                std::backtrace::Backtrace::capture()
            )
        } else {
            format!("{error:?}")
        };
        let exception_type = if expected {
            "EXPECTED EXCEPTION"
        } else {
            "UNEXPECTED EXCEPTION"
        };
        let log_message = match message {
            Some(message) => self.write_log(
                exception_type,
                &format!("{message} - {exception_title}\n{exception_message}"),
            ),
            None => self.write_log(
                exception_type,
                &format!("{exception_title}\n{exception_message}"),
            ),
        };
        if print_log {
            println!("{MAGENTA}{log_message}{RESET}");
        }
    }

    /// Port of `Logger.httprequest`.
    pub fn httprequest(&self, remote_addr: &str, method: &str, url: &str, status: u16) {
        let log_message = self.write_log(
            "REQUEST",
            &format!("{remote_addr} - {method} {url} - Status: {status}"),
        );
        println!("{CYAN}{log_message}{RESET}");
    }
}

static LOG: OnceLock<Logger> = OnceLock::new();
static UPDATER_LOG: OnceLock<Logger> = OnceLock::new();

/// Port of the module-level `log = Logger()` in `app/utils/logger.py`.
pub fn log() -> &'static Logger {
    LOG.get_or_init(|| Logger::new(false))
}

/// Port of the module-level `log = Logger(from_updater=True)` in
/// `app/updater/updater.py`.
pub fn updater_log() -> &'static Logger {
    UPDATER_LOG.get_or_init(|| Logger::new(true))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_date_conversion() {
        // 2024-09-26 00:00:00 UTC = 1727308800
        assert_eq!(days_to_civil_date(1727308800 / 86_400), (2024, 9, 26));
        assert_eq!(days_to_civil_date(0), (1970, 1, 1));
    }

    #[test]
    fn logger_writes_file() {
        let dir = std::env::temp_dir().join(format!("webdeck-log-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let logger = Logger::new(false);
        logger.set_log_file(&dir.join("test.log"));
        logger.disable_debug();
        logger.info("hello");
        logger.debug("hidden");
        logger.enable_debug();
        logger.debug("shown");
        let content = std::fs::read_to_string(dir.join("test.log")).unwrap();
        assert!(content.contains("[INFO @") && content.contains("hello"));
        assert!(content.contains("hidden") && content.contains("shown"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
