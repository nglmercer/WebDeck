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

/// psutil reports percents with one decimal (`round(x, 1)`); sysinfo math
/// yields full precision, which also overflows the narrow tiles.
fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn asked_main(asked_devices: &[Vec<String>], device: &str) -> bool {
    asked_devices
        .iter()
        .any(|item| item.first().map(|s| s == device).unwrap_or(false))
}

fn asked_metric(asked_devices: &[Vec<String>], level: usize, metric: &str) -> bool {
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

/// Linux-only eval-safe disk alias: device basename with every
/// non-identifier char mapped to `_` (`/dev/nvme0n1p4` → `nvme0n1p4`),
/// so usage tiles can address disks through the JS `eval` paths.
#[cfg(target_os = "linux")]
fn disk_alias_key(disk_name: &str) -> String {
    let base = disk_name.rsplit('/').next().unwrap_or(disk_name);
    let mut alias: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if alias.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        alias.insert(0, '_');
    }
    if alias.is_empty() {
        alias.push_str("disk");
    }
    alias
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
        let mut disks_map = serde_json::Map::new();
        for disk in Disks::new_with_refreshed_list().iter() {
            // Per-disk errors are swallowed like Python's try/except/pass.
            let raw_name = disk.name().to_string_lossy();
            let disk_name = raw_name.replace('\\', "").replace(':', "");
            // Linux-only: device paths are not valid JS eval paths, so the
            // basename alias is exposed (and matchable) alongside the 1:1 key.
            #[cfg(target_os = "linux")]
            let alias = disk_alias_key(&disk_name);
            #[cfg(target_os = "linux")]
            let wanted = get_all
                || asked_metric(asked_devices, 1, &disk_name)
                || (alias != disk_name && asked_metric(asked_devices, 1, &alias));
            #[cfg(not(target_os = "linux"))]
            let wanted = get_all || asked_metric(asked_devices, 1, &disk_name);
            if !wanted {
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
                entry.insert("usage_percent".to_string(), json!(round1(usage_percent)));
            }
            let value = Value::Object(entry);
            #[cfg(target_os = "linux")]
            if alias != disk_name {
                disks_map.insert(alias, value.clone());
            }
            disks_map.insert(disk_name, value);
        }
        computer_info["disks"] = Value::Object(disks_map);
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
    } else if method == "AMD" {
        // Linux-only sysfs backend (the UI already offers "AMD"); other
        // platforms keep the empty shape like Python.
        for (key, value) in amd_gpu_entries() {
            gpus.insert(key, value);
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

/// GPUtil-shaped entries for amdgpu cards (Linux-only). Fields that the
/// hardware/firmware does not expose are omitted — never zero-filled — so
/// tiles stay `-` instead of showing fake data.
#[cfg(target_os = "linux")]
fn amd_gpu_entries() -> Vec<(String, Value)> {
    let mut entries = Vec::new();
    match amd_devices() {
        Ok(devices) => {
            for (count, device) in devices.iter().enumerate() {
                let mut entry = serde_json::Map::new();
                entry.insert("name".to_string(), json!(device.name));
                if let (Some(used), Some(total)) = (device.used_mb, device.total_mb) {
                    entry.insert("used_mb".to_string(), json!(round2(used)));
                    entry.insert("total_mb".to_string(), json!(round2(total)));
                    entry.insert("available_mb".to_string(), json!(round2(total - used)));
                }
                if let Some(pct) = device.usage_percent {
                    entry.insert("usage_percent".to_string(), json!(pct));
                }
                if let Some(temp) = device.temperature_c {
                    entry.insert("temperature_c".to_string(), json!(round2(temp)));
                }
                entries.push((format!("GPU{}", count + 1), Value::Object(entry)));
            }
        }
        Err(e) => {
            log().debug(&format!("AMD GPU read failed: {e}"));
        }
    }
    entries
}

#[cfg(not(target_os = "linux"))]
fn amd_gpu_entries() -> Vec<(String, Value)> {
    log().debug("AMD GPU metrics are only supported on Linux");
    Vec::new()
}

struct AmdDeviceInfo {
    name: String,
    used_mb: Option<f64>,
    total_mb: Option<f64>,
    usage_percent: Option<i64>,
    temperature_c: Option<f64>,
}

#[cfg(target_os = "linux")]
fn amd_devices() -> Result<Vec<AmdDeviceInfo>, String> {
    let mut devices = Vec::new();
    let mut cards: Vec<String> = std::fs::read_dir("/sys/class/drm")
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| {
            n.strip_prefix("card").is_some_and(|rest| {
                !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
            })
        })
        .collect();
    cards.sort();
    for card in &cards {
        let dev = format!("/sys/class/drm/{card}/device");
        // Only amdgpu-driven cards (skips Intel iGD, virtio, ...).
        let driver = std::fs::read_link(format!("{dev}/driver"))
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !driver.ends_with("amdgpu") {
            continue;
        }
        let metrics =
            std::fs::read(format!("{dev}/gpu_metrics")).ok().and_then(|b| {
                parse_gpu_metrics(&b)
            });
        let usage_percent = read_u64_file(&format!("{dev}/gpu_busy_percent"))
            .filter(|pct| *pct <= 100)
            .map(|pct| pct as i64)
            .or_else(|| metrics.as_ref().and_then(|m| m.gfx_activity));
        devices.push(AmdDeviceInfo {
            name: format!("AMD GPU{}", devices.len() + 1),
            used_mb: read_u64_file(&format!("{dev}/mem_info_vram_used"))
                .map(|b| b as f64 / 1024.0 / 1024.0),
            total_mb: read_u64_file(&format!("{dev}/mem_info_vram_total"))
                .map(|b| b as f64 / 1024.0 / 1024.0),
            usage_percent,
            temperature_c: metrics
                .as_ref()
                .and_then(|m| m.temperature_c)
                .or_else(|| amd_hwmon_temp(&dev)),
        });
    }
    if devices.is_empty() {
        return Err("no amdgpu cards found".to_string());
    }
    Ok(devices)
}

/// Parsed subset of one `gpu_metrics` blob.
#[cfg(target_os = "linux")]
struct AmdMetrics {
    gfx_activity: Option<i64>,
    temperature_c: Option<f64>,
}

/// Parse `average_gfx_activity` + a headline temperature out of a raw
/// `gpu_metrics` blob. Layouts per the kernel's
/// `drivers/gpu/drm/amd/include/kgd_pp_interface.h`; v1.9 (attribute
/// array) and v2.0 (kernel-marked misaligned) are unsupported. Every
/// value is range-gated so unknown firmware revisions yield `None`,
/// never garbage.
#[cfg(target_os = "linux")]
fn parse_gpu_metrics(buf: &[u8]) -> Option<AmdMetrics> {
    let format = *buf.get(2)?;
    let content = *buf.get(3)?;
    // (activity offset, temp offset, temp in centidegrees)
    let (act_off, temp_off, centi) = match (format, content) {
        (1, 0) => (28, 16, true),
        (1, 1) | (1, 2) | (1, 3) => (16, 4, true),
        (1, 4) | (1, 5) | (1, 6) | (1, 7) | (1, 8) => (12, 4, false),
        (2, 1) | (2, 2) | (2, 3) | (2, 4) => (28, 4, true),
        (3, 0) => (42, 4, true),
        _ => return None,
    };
    let raw_act = u16::from_le_bytes(buf.get(act_off..act_off + 2)?.try_into().ok()?);
    let raw_temp = u16::from_le_bytes(buf.get(temp_off..temp_off + 2)?.try_into().ok()?);
    let gfx_activity = (raw_act <= 100).then_some(raw_act as i64);
    let temperature_c = if centi {
        (raw_temp > 0 && raw_temp < 15_000).then(|| raw_temp as f64 / 100.0)
    } else {
        (raw_temp > 0 && raw_temp < 150).then(|| raw_temp as f64)
    };
    Some(AmdMetrics {
        gfx_activity,
        temperature_c,
    })
}

/// hwmon `temp1_input` fallback (millidegree Celsius).
#[cfg(target_os = "linux")]
fn amd_hwmon_temp(dev: &str) -> Option<f64> {
    let hwmon = std::fs::read_dir(format!("{dev}/hwmon")).ok()?;
    for entry in hwmon.flatten() {
        let raw: Option<f64> = std::fs::read_to_string(entry.path().join("temp1_input"))
            .ok()
            .and_then(|s| s.trim().parse().ok());
        if let Some(c) = raw.map(|raw| raw / 1000.0) {
            if c > 0.0 && c < 150.0 {
                return Some((c * 100.0).round() / 100.0);
            }
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn read_u64_file(path: &str) -> Option<u64> {
    std::fs::read_to_string(path)
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
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
    fn disk_alias_is_eval_safe() {
        assert_eq!(disk_alias_key("/dev/nvme0n1p4"), "nvme0n1p4");
        assert_eq!(disk_alias_key("/dev/mapper/vg-lv"), "vg_lv");
        assert_eq!(disk_alias_key("C"), "C");
        assert_eq!(disk_alias_key("/"), "disk");
        assert_eq!(disk_alias_key("9lives"), "_9lives");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn parses_gpu_metrics_v2_2() {
        // Real captured prefix (BC-250): v2.2, gfx temp 6825 (=68.25C),
        // activity unsupported (0xFFFF).
        let mut blob = vec![0u8; 128];
        let prefix: [u8; 32] = [
            0x80, 0x00, 0x02, 0x02, 0xa9, 0x1a, 0xfa, 0x19, 0xe1, 0x19, 0x71,
            0x1b, 0x5e, 0x1a, 0xaf, 0x19, 0xc8, 0x19, 0x64, 0x19, 0xff, 0xff,
            0xff, 0xff, 0x13, 0x1a, 0x96, 0x19, 0xff, 0xff, 0xff, 0xff,
        ];
        blob[..32].copy_from_slice(&prefix);
        let m = parse_gpu_metrics(&blob).expect("v2.2 parses");
        assert_eq!(m.gfx_activity, None);
        assert_eq!(m.temperature_c, Some(68.25));
        // Activity present + whole-degree v1.4 layout.
        blob[28] = 42;
        blob[29] = 0;
        let m = parse_gpu_metrics(&blob).expect("v2.2 parses");
        assert_eq!(m.gfx_activity, Some(42));
        let mut v14 = vec![0u8; 116];
        v14[2] = 1;
        v14[3] = 4;
        v14[4] = 65;
        v14[12] = 7;
        let m = parse_gpu_metrics(&v14).expect("v1.4 parses");
        assert_eq!(m.gfx_activity, Some(7));
        assert_eq!(m.temperature_c, Some(65.0));
        // Unknown versions and truncated blobs yield None, never garbage.
        v14[3] = 9;
        assert!(parse_gpu_metrics(&v14).is_none());
        assert!(parse_gpu_metrics(&blob[..10]).is_none());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn amd_shape_on_amdgpu_hardware() {
        let has_amdgpu = std::fs::read_dir("/sys/class/drm")
            .map(|entries| {
                entries.flatten().any(|e| {
                    let n = e.file_name().to_string_lossy().into_owned();
                    n.starts_with("card")
                        && std::fs::read_link(format!(
                            "/sys/class/drm/{n}/device/driver"
                        ))
                        .map(|p| p.to_string_lossy().ends_with("amdgpu"))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        if !has_amdgpu {
            return;
        }
        let gpus = gpu_info(&serde_json::json!({"settings": {"gpu_method": "AMD"}}));
        let gpu1 = gpus.get("GPU1").expect("GPU1 on amdgpu hardware");
        assert!(gpu1.get("name").is_some());
        // This machine's chip reports VRAM + temp but no load counter.
        if let Some(total) = gpu1.get("total_mb").and_then(|v| v.as_f64()) {
            assert!(total > 0.0);
        }
        if let Some(temp) = gpu1.get("temperature_c").and_then(|v| v.as_f64()) {
            assert!((0.0..150.0).contains(&temp));
        }
        assert!(gpus.get("defaultGPU").is_some());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn live_disks_have_aliases_and_rounded_percents() {
        let _guard = test_env();
        let info = get_usage(Some(true), &[]);
        let disks = info.get("disks").and_then(|d| d.as_object()).expect("disks");
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
            assert!(
                (pct * 10.0).fract().abs() < 1e-6,
                "not 1-decimal: {pct}"
            );
        }
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
