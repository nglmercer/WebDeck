//! Port of `app/utils/debug/timers.py`.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::app::utils::logger::log;

/// Port of the `Timer` class in `app/utils/debug/timers.py`.
///
/// Python stores epoch floats; Rust stores `Instant`s (monotonic, no clock
/// jumps). Interior mutability lets the global [`timer()`] share the same
/// call shape as Python's module-level `timer = Timer()`.
pub struct Timer {
    name: String,
    timers: Mutex<HashMap<String, Instant>>,
    stopped_timers: Mutex<HashMap<String, Duration>>,
}

impl Timer {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            timers: Mutex::new(HashMap::new()),
            stopped_timers: Mutex::new(HashMap::new()),
        }
    }

    fn format_elapsed(tag: &str, name: &str, elapsed: Duration) -> String {
        let total_secs = elapsed.as_secs();
        let (minutes, seconds) = (total_secs / 60, total_secs % 60);
        let milliseconds = elapsed.subsec_millis();
        format!(
            "[{tag}] Timer '{name}' {} {minutes} minutes, {seconds} seconds, {milliseconds} milliseconds",
            match tag {
                "S" => "started at",
                "G" | "Final" => "is at",
                _ => "stopped at",
            }
        )
    }

    /// Port of `Timer.start`.
    pub fn start(&self, name: &str, logger: bool) {
        if let Ok(mut timers) = self.timers.lock() {
            timers.insert(name.to_string(), Instant::now());
            if logger {
                log().debug(&format!("[S] Timer '{name}' started"));
            }
        }
    }

    fn elapsed_locked(
        &self,
        name: &str,
        timers: &HashMap<String, Instant>,
        stopped: &HashMap<String, Duration>,
    ) -> Result<Duration, String> {
        if let Some(start) = timers.get(name) {
            Ok(start.elapsed())
        } else if let Some(elapsed) = stopped.get(name) {
            Ok(*elapsed)
        } else {
            Err(format!("Timer '{name}' does not exist."))
        }
    }

    /// Port of `Timer.get` (returns elapsed seconds).
    pub fn get(&self, name: &str, logger: bool) -> Result<f64, String> {
        let timers = self.timers.lock().map_err(|e| e.to_string())?;
        let stopped = self.stopped_timers.lock().map_err(|e| e.to_string())?;
        let elapsed = self.elapsed_locked(name, &timers, &stopped)?;
        if logger {
            log().debug(&Self::format_elapsed("G", name, elapsed));
        }
        Ok(elapsed.as_secs_f64())
    }

    /// Port of `Timer.stop` (returns elapsed seconds).
    pub fn stop(&self, name: &str, logger: bool) -> Result<f64, String> {
        let mut timers = self.timers.lock().map_err(|e| e.to_string())?;
        let start = timers
            .remove(name)
            .ok_or_else(|| format!("Timer '{name}' does not exist."))?;
        let elapsed = start.elapsed();
        if let Ok(mut stopped) = self.stopped_timers.lock() {
            stopped.insert(name.to_string(), elapsed);
        }
        if logger {
            log().debug(&Self::format_elapsed("/", name, elapsed));
        }
        Ok(elapsed.as_secs_f64())
    }

    /// Port of `Timer.stop_all`.
    pub fn stop_all(&self, logger: bool) -> HashMap<String, f64> {
        let names: Vec<String> = self
            .timers
            .lock()
            .map(|timers| timers.keys().cloned().collect())
            .unwrap_or_default();
        let mut times = HashMap::new();
        for name in names {
            if let Ok(elapsed) = self.stop(&name, logger) {
                times.insert(name, elapsed);
            }
        }
        times
    }

    /// Port of `Timer.get_longest_timers`.
    pub fn get_longest_timers(&self, count: usize, logger: bool) -> Vec<(String, f64)> {
        let snapshot: Vec<(String, Duration)> = {
            let timers = self.timers.lock();
            let stopped = self.stopped_timers.lock();
            let mut all: Vec<(String, Duration)> = Vec::new();
            if let Ok(timers) = timers.as_ref() {
                for (name, start) in timers.iter() {
                    all.push((name.clone(), start.elapsed()));
                }
            }
            if let Ok(stopped) = stopped.as_ref() {
                for (name, elapsed) in stopped.iter() {
                    all.push((name.clone(), *elapsed));
                }
            }
            all
        };
        if snapshot.is_empty() {
            return Vec::new();
        }
        let mut sorted = snapshot;
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted
            .into_iter()
            .take(count)
            .map(|(name, elapsed)| {
                if logger {
                    log().debug(&Self::format_elapsed("Final", &name, elapsed));
                }
                (name, elapsed.as_secs_f64())
            })
            .collect()
    }
}

static TIMER: OnceLock<Timer> = OnceLock::new();

/// Port of the module-level `timer = Timer()` in `app/utils/debug/timers.py`.
pub fn timer() -> &'static Timer {
    TIMER.get_or_init(|| Timer::new("default"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_get_stop() {
        let timer = Timer::new("test");
        timer.start("a", false);
        let elapsed = timer.get("a", false).unwrap();
        assert!(elapsed >= 0.0);
        let stopped = timer.stop("a", false).unwrap();
        assert!(stopped >= 0.0);
        assert!(timer.get("missing", false).is_err());
        assert!(timer.stop("missing", false).is_err());
    }

    #[test]
    fn stop_all_and_longest() {
        let timer = Timer::new("test");
        timer.start("x", false);
        timer.start("y", false);
        std::thread::sleep(Duration::from_millis(2));
        let longest = timer.get_longest_timers(1, false);
        assert_eq!(longest.len(), 1);
        let times = timer.stop_all(false);
        assert_eq!(times.len(), 2);
        assert!(timer.get_longest_timers(5, false).len() == 2);
    }
}
