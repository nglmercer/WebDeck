//! Port of `app/buttons/usage/get_usage.py`.
//!
//! - `psutil` → `sysinfo` (CPU/memory/disks/network).
//! - `pynvml`/`GPUtil` → `nvml-wrapper` (both Python paths are NVML-based,
//!   so they share one source; field shapes stay per-branch).
//! - The `get_all`/`asked_devices` gating, disk-name mangling, and the
//!   final `merge_dicts(get_usage(True), …)` merge are ported 1:1.
//! - Like `psutil.cpu_percent()`, the first CPU reading is `0.0` (needs a
//!   previous sample); a process-wide `System` keeps later readings live.

use std::sync::{Mutex, OnceLock};

use serde_json::{json, Value};
use sysinfo::{Disks, Networks, System};

use crate::app::buttons::usage::asked_devices::get_asked_devices;
use crate::app::utils::{
    logger::log, merge_dicts::merge_dicts, settings::get_config::get_config,
    settings::save_config::save_config,
};

static SYS: OnceLock<Mutex<System>> = OnceLock::new();

fn system() -> &'static Mutex<System> {
    SYS.get_or_init(|| Mutex::new(System::new_all()))
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn asked_main(asked_devices: &[Vec<String>], device: &str) -> bool {
    asked_devices
        .iter()
        .any(|item| item.first().map(|s| s == device).unwrap_or(false))
}

fn asked_metric(asked_devices: &[Vec<String>], level: usize, metric: &str) -> bool {
    asked_devices.iter().any(|item| {
        item.len() == 3 && level == 2 && item.get(2).map(|s| s == metric).unwrap_or(false)
            || item.len() != 3 && item.get(level).map(|s| s == metric).unwrap_or(false)
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
        computer_info["cpu"] = json!({"usage_percent": usage_percent});
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
            memory.insert("usage_percent".to_string(), json!(usage_percent));
        }
        computer_info["memory"] = Value::Object(memory);
    }

    // Hard disk
    if get_all || asked_main(asked_devices, "disks") {
        let mut disks_map = serde_json::Map::new();
        for disk in Disks::new_with_refreshed_list().iter() {
            // Per-disk errors are swallowed like Python's try/except/pass.
            let raw_name = disk.name().to_string_lossy();
            let disk_name = raw_name.replace('\\', "").replace(':', "");
            if !(get_all || asked_metric(asked_devices, 1, &disk_name)) {
                continue;
            }
            let total = disk.total_space() as f64;
            let free = disk.available_space() as f64;
            let used = total - free;
            let usage_percent = if total > 0.0 {
                used / total * 100.0
            } else {
                0.0
            };
            let mut entry = serde_json::Map::new();
            if get_all || asked_metric(asked_devices, 2, "total_gb") {
                entry.insert(
                    "total_gb".to_string(),
                    json!(round2(total / 1024f64.powi(3))),
                );
            }
            if get_all || asked_metric(asked_devices, 2, "used_gb") {
                entry.insert("used_gb".to_string(), json!(round2(used / 1024f64.powi(3))));
            }
            if get_all || asked_metric(asked_devices, 2, "free_gb") {
                entry.insert("free_gb".to_string(), json!(round2(free / 1024f64.powi(3))));
            }
            if get_all || asked_metric(asked_devices, 2, "usage_percent") {
                entry.insert("usage_percent".to_string(), json!(usage_percent));
            }
            disks_map.insert(disk_name, Value::Object(entry));
        }
        computer_info["disks"] = Value::Object(disks_map);
    }

    // Network
    if get_all || asked_main(asked_devices, "network") {
        let mut bytes_sent: u64 = 0;
        let mut bytes_recv: u64 = 0;
        for (_, data) in Networks::new_with_refreshed_list().iter() {
            bytes_sent += data.transmitted();
            bytes_recv += data.received();
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

/// Port of the GPU branch of `get_usage` (both `nvidia` methods via NVML).
fn gpu_info(config: &Value) -> Value {
    let method = config
        .get("settings")
        .and_then(|s| s.get("gpu_method"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let mut gpus = serde_json::Map::new();

    if method == "nvidia (pynvml)" {
        match nvml_devices() {
            Ok(devices) => {
                for (count, device) in devices.iter().enumerate() {
                    gpus.insert(
                        format!("GPU{}", count + 1),
                        json!({"usage_percent": device.usage_percent}),
                    );
                }
            }
            Err(_) => {
                // Unsupported graphics cards (mirrors the except branch).
                gpus.insert("defaultGPU".to_string(), json!({}));
                let mut config = config.clone();
                if let Some(settings) = config.get_mut("settings").and_then(|s| s.as_object_mut()) {
                    settings.insert("gpu_method".to_string(), json!("None"));
                }
                save_config(config);
            }
        }
    } else if method == "nvidia (GPUtil)" {
        match nvml_devices() {
            Ok(devices) => {
                for (count, device) in devices.iter().enumerate() {
                    gpus.insert(
                        format!("GPU{}", count + 1),
                        json!({
                            "name": device.name,
                            "used_mb": device.used_mb,
                            "total_mb": device.total_mb,
                            "available_mb": device.total_mb - device.used_mb,
                            "usage_percent": device.usage_percent,
                        }),
                    );
                }
            }
            Err(e) => {
                log().debug(&format!("GPU read failed: {e}"));
            }
        }
    } else {
        gpus.insert("defaultGPU".to_string(), json!({}));
    }

    if let Some(gpu1) = gpus.get("GPU1").cloned() {
        gpus.insert("defaultGPU".to_string(), gpu1);
    }
    Value::Object(gpus)
}

struct NvmlDeviceInfo {
    name: String,
    used_mb: f64,
    total_mb: f64,
    usage_percent: i64,
}

fn nvml_devices() -> Result<Vec<NvmlDeviceInfo>, String> {
    let nvml = nvml_wrapper::Nvml::init().map_err(|e| e.to_string())?;
    let count = nvml.device_count().map_err(|e| e.to_string())?;
    let mut devices = Vec::new();
    for i in 0..count {
        let device = nvml.device_by_index(i).map_err(|e| e.to_string())?;
        let utilization = device.utilization_rates().map_err(|e| e.to_string())?;
        let memory = device.memory_info().map_err(|e| e.to_string())?;
        let name = device.name().unwrap_or_else(|_| format!("GPU{i}"));
        devices.push(NvmlDeviceInfo {
            name,
            used_mb: memory.used as f64 / 1024.0 / 1024.0,
            total_mb: memory.total as f64 / 1024.0 / 1024.0,
            usage_percent: utilization.gpu as i64,
        });
    }
    Ok(devices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::utils::settings::get_config::test_support::{config_guard, seed_config};

    /// Serialized temp-config env, re-seeded every time so tests stay
    /// order-independent.
    fn test_env() -> std::sync::MutexGuard<'static, ()> {
        let guard = config_guard();
        seed_config(&serde_json::json!({
            "url": {"port": 5000},
            "front": {"buttons": {}},
            "settings": {
                "optimized_usage_display": false,
                "gpu_method": "nvidia (pynvml)",
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

    #[test]
    fn filtered_merges_with_full() {
        let _guard = test_env();
        let info = get_usage(Some(false), &[vec!["cpu".to_string()]]);
        assert!(info.get("cpu").is_some());
        // The merge_dicts(get_usage(True), …) tail keeps every section.
        assert!(info.get("memory").is_some());
    }
}
