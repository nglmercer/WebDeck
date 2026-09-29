//! Linux sysfs AMD GPU backend (extracted from `get_usage.rs`).
//!
//! Non-Linux platforms keep the empty shape via the stub below, like
//! Python (which has no AMD path at all).

#[cfg(target_os = "linux")]
use super::get_usage::round2;
#[cfg(target_os = "linux")]
use serde_json::json;
use serde_json::Value;

use crate::app::utils::logger::log;

/// GPUtil-shaped entries for amdgpu cards (Linux-only). Fields that the
/// hardware/firmware does not expose are omitted — never zero-filled — so
/// tiles stay `-` instead of showing fake data.
#[cfg(target_os = "linux")]
pub(crate) fn amd_gpu_entries() -> Vec<(String, Value)> {
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
pub(crate) fn amd_gpu_entries() -> Vec<(String, Value)> {
    log().debug("AMD GPU metrics are only supported on Linux");
    Vec::new()
}

/// `cardN` DRM entries driven by amdgpu (sorted) — the shared
/// card-discovery behind [`has_amdgpu_card`] and [`amd_devices`].
#[cfg(target_os = "linux")]
fn amdgpu_cards() -> Result<Vec<String>, String> {
    let mut cards: Vec<String> = std::fs::read_dir("/sys/class/drm")
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| {
            n.strip_prefix("card")
                .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
                && std::fs::read_link(format!("/sys/class/drm/{n}/device/driver"))
                    .map(|p| p.to_string_lossy().ends_with("amdgpu"))
                    .unwrap_or(false)
        })
        .collect();
    cards.sort();
    Ok(cards)
}

/// Whether an amdgpu-driven card exists (Linux sysfs probe, no metrics
/// read). Used by startup recovery to pick a working `gpu_method`.
#[cfg(target_os = "linux")]
pub(crate) fn has_amdgpu_card() -> bool {
    amdgpu_cards().is_ok_and(|cards| !cards.is_empty())
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn has_amdgpu_card() -> bool {
    false
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
    // Already amdgpu-only (skips Intel iGD, virtio, ...).
    let cards = amdgpu_cards()?;
    for card in &cards {
        let dev = format!("/sys/class/drm/{card}/device");
        let metrics = std::fs::read(format!("{dev}/gpu_metrics"))
            .ok()
            .and_then(|b| parse_gpu_metrics(&b));
        let pdev = std::fs::read_to_string(format!("{dev}/uevent"))
            .ok()
            .and_then(|u| {
                u.lines().find_map(|l| {
                    l.strip_prefix("PCI_SLOT_NAME=")
                        .map(|s| s.trim().to_string())
                })
            })
            .unwrap_or_default();
        let usage_percent = read_u64_file(&format!("{dev}/gpu_busy_percent"))
            .filter(|pct| *pct <= 100)
            .map(|pct| pct as i64)
            .or_else(|| {
                if pdev.is_empty() {
                    None
                } else {
                    amd_fdinfo_load_percent(&pdev)
                }
            })
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
        (raw_temp > 0 && raw_temp < 150).then_some(raw_temp as f64)
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

/// Last fdinfo GFX sample per PCI device (`total_ns`, timestamp).
#[cfg(target_os = "linux")]
fn amd_gfx_cache(
) -> &'static std::sync::Mutex<std::collections::HashMap<String, (u64, std::time::Instant)>> {
    static CACHE: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<String, (u64, std::time::Instant)>>,
    > = std::sync::OnceLock::new();
    CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

/// Load % from two GFX-time samples. `None` on zero/out-of-order elapsed
/// time or a counter reset (client churn between polls). Concurrent
/// clients can legitimately sum past wall time, so the result clamps to
/// 100 instead of rejecting.
#[cfg(target_os = "linux")]
fn gfx_load_percent(
    prev_ns: u64,
    prev_t: std::time::Instant,
    cur_ns: u64,
    cur_t: std::time::Instant,
) -> Option<i64> {
    let elapsed = cur_t.checked_duration_since(prev_t)?.as_nanos();
    if elapsed == 0 {
        return None;
    }
    let delta = cur_ns.checked_sub(prev_ns)?;
    Some(
        (delta as f64 / elapsed as f64 * 100.0)
            .round()
            .clamp(0.0, 100.0) as i64,
    )
}

/// GFX load % for one amdgpu PCI device, measured from the DRM
/// scheduler's per-client `drm-engine-gfx` counters in
/// `/proc/*/fdinfo/*`. Fallback for chips whose firmware exposes no
/// `gpu_busy_percent`/`gfx_activity` counter (e.g. BC-250, where the
/// sysfs node reads back EOPNOTSUPP and the metrics blob says 0xFFFF).
/// The counters are scheduler-side, so this is real measured load — but
/// only same-user processes' fdinfo is readable, so GPU clients owned by
/// other users are invisible and load undercounts accordingly.
///
/// The first call per device primes the cache and yields `None`;
/// callers must tolerate that (`-` until the second poll).
#[cfg(target_os = "linux")]
fn amd_fdinfo_load_percent(pdev: &str) -> Option<i64> {
    let current = amd_gfx_total_ns(pdev)?;
    let now = std::time::Instant::now();
    let previous = amd_gfx_cache()
        .lock()
        .ok()?
        .insert(pdev.to_string(), (current, now));
    let (prev_ns, prev_t) = previous?;
    gfx_load_percent(prev_ns, prev_t, current, now)
}

/// Sum of `drm-engine-gfx` nanoseconds across distinct amdgpu DRM clients
/// of one PCI device. Clients dedup by (`drm-pdev`, `drm-client-id`) so
/// dup'd descriptors count once; entries the kernel leaves unattributed
/// degrade to per-file counting. `None` when no client stats exist.
#[cfg(target_os = "linux")]
fn amd_gfx_total_ns(pdev: &str) -> Option<u64> {
    let mut clients: std::collections::HashMap<(String, String), u64> =
        std::collections::HashMap::new();
    for pid in std::fs::read_dir("/proc").ok()?.flatten() {
        let pid_name = pid.file_name().to_string_lossy().into_owned();
        if pid_name.is_empty() || !pid_name.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let fdinfos = std::fs::read_dir(format!("/proc/{pid_name}/fdinfo"));
        for fd in fdinfos.ok().into_iter().flatten().flatten() {
            let content = std::fs::read_to_string(fd.path()).unwrap_or_default();
            if !content.contains("drm-engine-gfx:") {
                continue;
            }
            let mut driver = "";
            let mut dev = "";
            let mut client = "";
            let mut gfx = None;
            for line in content.lines() {
                if let Some(v) = line.strip_prefix("drm-driver:") {
                    driver = v.trim();
                } else if let Some(v) = line.strip_prefix("drm-pdev:") {
                    dev = v.trim();
                } else if let Some(v) = line.strip_prefix("drm-client-id:") {
                    client = v.trim();
                } else if let Some(v) = line.strip_prefix("drm-engine-gfx:") {
                    gfx = v
                        .split_whitespace()
                        .next()
                        .and_then(|n| n.parse::<u64>().ok());
                }
            }
            let gfx = match gfx {
                Some(g) => g,
                None => continue,
            };
            if driver != "amdgpu" || (!dev.is_empty() && dev != pdev) {
                continue;
            }
            let key = if client.is_empty() {
                // Kernels without client ids: per-file counting (dup'd
                // fds may double-count).
                (
                    format!("{pid_name}/{}", fd.file_name().to_string_lossy()),
                    String::new(),
                )
            } else {
                (dev.to_string(), client.to_string())
            };
            clients.insert(key, gfx);
        }
    }
    if clients.is_empty() {
        return None;
    }
    Some(clients.values().sum())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn parses_gpu_metrics_v2_2() {
        // Real captured prefix (BC-250): v2.2, gfx temp 6825 (=68.25C),
        // activity unsupported (0xFFFF).
        let mut blob = vec![0u8; 128];
        let prefix: [u8; 32] = [
            0x80, 0x00, 0x02, 0x02, 0xa9, 0x1a, 0xfa, 0x19, 0xe1, 0x19, 0x71, 0x1b, 0x5e, 0x1a,
            0xaf, 0x19, 0xc8, 0x19, 0x64, 0x19, 0xff, 0xff, 0xff, 0xff, 0x13, 0x1a, 0x96, 0x19,
            0xff, 0xff, 0xff, 0xff,
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
    fn gfx_load_percent_handles_edges() {
        use std::time::{Duration, Instant};
        let t0 = Instant::now();
        let t1 = t0 + Duration::from_secs(1);
        assert_eq!(gfx_load_percent(0, t0, 500_000_000, t1), Some(50));
        assert_eq!(gfx_load_percent(0, t0, 0, t0), None);
        assert_eq!(gfx_load_percent(100, t0, 50, t1), None);
        assert_eq!(gfx_load_percent(0, t0, 3_000_000_000, t1), Some(100));
        assert_eq!(gfx_load_percent(0, t1, 0, t0), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn fdinfo_sampler_yields_range_gated_load() {
        let cards = std::fs::read_dir("/sys/class/drm")
            .map(|entries| {
                entries
                    .flatten()
                    .filter_map(|e| {
                        let n = e.file_name().to_string_lossy().into_owned();
                        let uevent =
                            std::fs::read_to_string(format!("/sys/class/drm/{n}/device/uevent"))
                                .ok()?;
                        uevent.lines().find_map(|l| {
                            l.strip_prefix("PCI_SLOT_NAME=")
                                .map(|s| s.trim().to_string())
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let Some(pdev) = cards.into_iter().next() else {
            return;
        };
        let _ = amd_fdinfo_load_percent(&pdev);
        std::thread::sleep(std::time::Duration::from_millis(50));
        if let Some(pct) = amd_fdinfo_load_percent(&pdev) {
            assert!((0..=100).contains(&pct), "out of range: {pct}");
        }
        // None is fine too (no same-user GPU clients on this box).
    }
}
