//! Port of `app/buttons/usage/get_usage.py`.
//!
//! - `psutil` → `sysinfo` (CPU/memory/disks/network).
//! - `pynvml`/`GPUtil` → `nvml-wrapper` (both Python paths are NVML-based,
//!   so they share one source; field shapes stay per-branch).
//! - The `get_all`/`asked_devices` gating, disk-name mangling, and the
//!   final `merge_dicts(get_usage(True), …)` merge are ported 1:1.
//! - Like `psutil.cpu_percent()`, the first CPU reading is `0.0` (needs a
//!   previous sample); a process-wide `System` keeps later readings live.
//!
//! Section collectors live in sibling modules ([`super::disks`],
//! [`super::gpu`]); this file keeps the gating, CPU/memory/network
//! collectors, and the merge tail.

use std::sync::{Mutex, OnceLock};

use serde_json::{json, Value};
use sysinfo::{Networks, System};

use super::disks::collect_disks;
use super::gpu::gpu_info;
use crate::app::buttons::usage::asked_devices::get_asked_devices;
use crate::app::utils::{merge_dicts::merge_dicts, settings::get_config::get_config};

static SYS: OnceLock<Mutex<System>> = OnceLock::new();

fn system() -> &'static Mutex<System> {
    SYS.get_or_init(|| Mutex::new(System::new_all()))
}

pub(crate) fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// psutil reports percents with one decimal (`round(x, 1)`); sysinfo math
/// yields full precision, which also overflows the narrow tiles.
pub(crate) fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn asked_main(asked_devices: &[Vec<String>], device: &str) -> bool {
    asked_devices
        .iter()
        .any(|item| item.first().map(|s| s == device).unwrap_or(false))
}

pub(crate) fn asked_metric(asked_devices: &[Vec<String>], level: usize, metric: &str) -> bool {
    asked_devices.iter().any(|item| {
        // Python guards only the level-2 disk-metric check with
        // `len(item) == 3`; level-1 checks compare item[level] directly
        // (here via .get(), so short items miss instead of raising).
        if level == 2 {
            item.len() == 3 && item.get(2).map(|s| s == metric).unwrap_or(false)
        } else {
            item.get(level).map(|s| s == metric).unwrap_or(false)
        }
    })
}

/// Port of `get_usage`.
pub fn get_usage(get_all: Option<bool>, asked_devices: &[Vec<String>]) -> Value {
    let config = get_config(false, false);
    let get_all = get_all.unwrap_or_else(|| {
        !config
            .get("settings")
            .and_then(|s| s.get("optimized_usage_display"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    });

    let owned_devices: Vec<Vec<String>>;
    let asked_devices: &[Vec<String>] = if asked_devices.is_empty() && !get_all {
        owned_devices = get_asked_devices();
        &owned_devices
    } else {
        asked_devices
    };

    let mut computer_info = json!({});

    // CPU
    if get_all || asked_main(asked_devices, "cpu") {
        let usage_percent = system()
            .lock()
            .map(|mut sys| {
                sys.refresh_cpu_all();
                sys.global_cpu_usage() as f64
            })
            .unwrap_or(0.0);
        computer_info["cpu"] = json!({"usage_percent": round1(usage_percent)});
    }

    // Memory
    if get_all || asked_main(asked_devices, "memory") {
        let (total, available) = system()
            .lock()
            .map(|mut sys| {
                sys.refresh_memory();
                (sys.total_memory(), sys.available_memory())
            })
            .unwrap_or((0, 0));
        let total_gb = total as f64 / 1024f64.powi(3);
        let available_gb = available as f64 / 1024f64.powi(3);
        // NOTE: Python computes used as total − available (not used_memory).
        let used_gb = total_gb - available_gb;
        let usage_percent = if total > 0 {
            (total - available) as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        let mut memory = serde_json::Map::new();
        if get_all || asked_metric(asked_devices, 1, "total_gb") {
            memory.insert("total_gb".to_string(), json!(round2(total_gb)));
        }
        if get_all || asked_metric(asked_devices, 1, "used_gb") {
            memory.insert("used_gb".to_string(), json!(round2(used_gb)));
        }
        if get_all || asked_metric(asked_devices, 1, "available_gb") {
            memory.insert("available_gb".to_string(), json!(round2(available_gb)));
        }
        if get_all || asked_metric(asked_devices, 1, "usage_percent") {
            memory.insert("usage_percent".to_string(), json!(round1(usage_percent)));
        }
        computer_info["memory"] = Value::Object(memory);
    }

    // Hard disk
    if get_all || asked_main(asked_devices, "disks") {
        computer_info["disks"] = collect_disks(get_all, asked_devices);
    }

    // Network
    if get_all || asked_main(asked_devices, "network") {
        let mut bytes_sent: u64 = 0;
        let mut bytes_recv: u64 = 0;
        for (_, data) in Networks::new_with_refreshed_list().iter() {
            // NOTE: psutil reports lifetime totals; sysinfo's
            // transmitted()/received() are since-last-refresh (0 here).
            bytes_sent += data.total_transmitted();
            bytes_recv += data.total_received();
        }
        computer_info["network"] = json!({
            "bytes_sent": bytes_sent,
            "bytes_recv": bytes_recv,
        });
    }

    // GPU
    if get_all || asked_main(asked_devices, "gpus") {
        computer_info["gpus"] = gpu_info(&config);
    }

    if !get_all {
        computer_info = merge_dicts(get_usage(Some(true), &[]), &computer_info);
    }
    computer_info
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "linux")]
    use crate::app::buttons::usage::disks::disk_alias_key;
    use crate::app::utils::settings::get_config::test_support::{config_guard, seed_config};
    #[cfg(target_os = "linux")]
    use sysinfo::Disks;

    /// Serialized temp-config env, re-seeded every time so tests stay
    /// order-independent.
    fn test_env() -> std::sync::MutexGuard<'static, ()> {
        let guard = config_guard();
        seed_config(&serde_json::json!({
            "url": {"port": 5000},
            "front": {"buttons": {}},
            "settings": {
                "optimized_usage_display": false,
                "gpu_method": "nvidia (NVML)",
            },
        }));
        guard
    }

    #[test]
    fn full_shape_has_expected_sections() {
        let _guard = test_env();
        let info = get_usage(Some(true), &[]);
        for section in ["cpu", "memory", "disks", "network", "gpus"] {
            assert!(info.get(section).is_some(), "missing {section}");
        }
        // Sanity ranges (also validates sysinfo units are bytes).
        let total_gb = info["memory"]["total_gb"].as_f64().unwrap_or(0.0);
        assert!((0.5..=4096.0).contains(&total_gb), "total_gb={total_gb}");
        assert!(info["gpus"].get("defaultGPU").is_some());
    }

    /// Linux oracle: `/proc/net/dev` lifetime totals must match the
    /// reported network counters (guards against since-refresh mixups).
    #[cfg(target_os = "linux")]
    #[test]
    fn network_reports_lifetime_totals() {
        let _guard = test_env();
        let proc = std::fs::read_to_string("/proc/net/dev").expect("/proc/net/dev");
        let mut proc_recv: u64 = 0;
        let mut proc_sent: u64 = 0;
        for line in proc.lines().skip(2) {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() >= 10 {
                proc_recv += fields[1].parse::<u64>().unwrap_or(0);
                proc_sent += fields[9].parse::<u64>().unwrap_or(0);
            }
        }
        let info = get_usage(Some(true), &[]);
        let recv = info["network"]["bytes_recv"].as_u64().unwrap_or(0);
        let sent = info["network"]["bytes_sent"].as_u64().unwrap_or(0);
        // Background traffic moves between the two reads; tolerate it.
        let tolerance = (proc_recv / 100).max(10_000_000);
        assert!(
            recv.abs_diff(proc_recv) <= tolerance,
            "recv={recv} proc={proc_recv}"
        );
        let tolerance = (proc_sent / 100).max(10_000_000);
        assert!(
            sent.abs_diff(proc_sent) <= tolerance,
            "sent={sent} proc={proc_sent}"
        );
    }

    #[test]
    fn rounds_like_psutil() {
        assert_eq!(round1(52.69854263871834), 52.7);
        assert_eq!(round1(11.843202590942383), 11.8);
        assert_eq!(round1(100.0), 100.0);
    }

    #[test]
    fn asked_metric_matches_python_len_rules() {
        let len3 = vec![vec![
            "disks".to_string(),
            "C".to_string(),
            "usage_percent".to_string(),
        ]];
        // Python's level-1 check has no len guard (3-element disk items
        // match by name); only level-2 requires len == 3.
        assert!(asked_metric(&len3, 1, "C"));
        assert!(asked_metric(&len3, 2, "usage_percent"));
        assert!(!asked_metric(&len3, 2, "total_gb"));
        let short = vec![vec!["cpu".to_string()]];
        assert!(!asked_metric(&short, 1, "total_gb"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn live_disks_have_aliases_and_rounded_percents() {
        let _guard = test_env();
        let info = get_usage(Some(true), &[]);
        let disks = info
            .get("disks")
            .and_then(|d| d.as_object())
            .expect("disks");
        for key in disks.keys() {
            if key.contains('/') {
                assert!(
                    disks.contains_key(&disk_alias_key(key)),
                    "missing alias for {key}"
                );
            }
        }
        for pct in [
            info["cpu"]["usage_percent"].as_f64(),
            info["memory"]["usage_percent"].as_f64(),
        ] {
            let pct = pct.expect("percent present");
            assert!((pct * 10.0).fract().abs() < 1e-6, "not 1-decimal: {pct}");
        }
        // Windows-authored "C disk" tiles resolve against the root fs.
        let has_root = Disks::new_with_refreshed_list()
            .iter()
            .any(|d| d.mount_point() == std::path::Path::new("/"));
        if has_root {
            assert!(
                disks.contains_key("C"),
                "root filesystem missing \"C\" alias: {:?}",
                disks.keys().collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn failed_nvml_read_keeps_configured_method() {
        if nvml_wrapper::Nvml::init().is_ok() {
            return;
        }
        let _guard = test_env();
        let _ = get_usage(Some(true), &[]);
        // A failed NVML read must not persist "None" over the configured
        // method (that bricks GPU tiles with no recovery path).
        let after = get_config(false, false);
        assert_eq!(after["settings"]["gpu_method"], "nvidia (NVML)");
    }

    #[test]
    fn filtered_merges_with_full() {
        let _guard = test_env();
        let info = get_usage(Some(false), &[vec!["cpu".to_string()]]);
        assert!(info.get("cpu").is_some());
        // The merge_dicts(get_usage(True), …) tail keeps every section.
        assert!(info.get("memory").is_some());
    }
}
