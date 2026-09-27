//! GPU section of `get_usage` (extracted from `get_usage.rs`).
//!
//! The `gpu_method` dispatcher plus the shared NVML backend live here;
//! the Linux sysfs AMD backend is in [`super::gpu_amd`].

use serde_json::{json, Value};

use super::gpu_amd::{amd_gpu_entries, has_amdgpu_card};
use crate::app::utils::logger::log;

/// Port of the GPU branch of `get_usage` (both `nvidia` methods via NVML).
pub(crate) fn gpu_info(config: &Value) -> Value {
    let configured = config
        .get("settings")
        .and_then(|s| s.get("gpu_method"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    // A stuck `"None"` (or empty/unknown) method must not brick GPU tiles:
    // stale UI saves can re-persist it at any time, so probe for a working
    // backend on the read path instead of reporting empty. "Intel" stays
    // empty (explicit choice, no backend).
    let method = match configured {
        "nvidia (pynvml)" | "nvidia (GPUtil)" | "AMD" | "Intel" => configured.to_string(),
        _ => {
            if nvml_wrapper::Nvml::init().is_ok() {
                "nvidia (pynvml)".to_string()
            } else if has_amdgpu_card() {
                "AMD".to_string()
            } else {
                configured.to_string()
            }
        }
    };
    let method = method.as_str();
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
            Err(e) => {
                // Unsupported graphics cards. Python rewrote
                // `gpu_method` to "None" here, permanently bricking GPU
                // tiles (nothing ever flips it back); just report empty
                // and leave the configured method alone.
                log().debug(&format!("GPU read failed: {e}"));
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
    }

    // Every method yields at least an (empty) `defaultGPU` so the section
    // shape — and the tile lookup — is stable even with no GPU data.
    if gpus.is_empty() {
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

    #[test]
    fn default_gpu_key_always_present() {
        // Every method — including failing NVML and card-less AMD — must
        // yield `defaultGPU` so tile lookups hit a stable shape.
        for method in [
            "nvidia (pynvml)",
            "nvidia (GPUtil)",
            "AMD",
            "Intel",
            "None",
            "",
            "bogus",
        ] {
            let gpus = gpu_info(&serde_json::json!({"settings": {"gpu_method": method}}));
            assert!(
                gpus.get("defaultGPU").is_some(),
                "method {method} lost defaultGPU"
            );
        }
    }

    #[test]
    fn none_method_falls_back_to_working_backend() {
        let gpus = gpu_info(&serde_json::json!({"settings": {"gpu_method": "None"}}));
        if nvml_wrapper::Nvml::init().is_ok() {
            // NVML answers: pynvml-shaped entries (or an empty defaultGPU
            // when the driver reports zero devices).
            assert!(gpus.get("GPU1").is_some() || gpus.get("defaultGPU") == Some(&json!({})));
        } else if has_amdgpu_card() {
            assert!(
                gpus.get("GPU1").is_some(),
                "None should fall back to AMD entries, got {gpus}"
            );
        } else {
            assert_eq!(gpus.get("defaultGPU"), Some(&json!({})));
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn amd_shape_on_amdgpu_hardware() {
        let has_amdgpu = std::fs::read_dir("/sys/class/drm")
            .map(|entries| {
                entries.flatten().any(|e| {
                    let n = e.file_name().to_string_lossy().into_owned();
                    n.starts_with("card")
                        && std::fs::read_link(format!("/sys/class/drm/{n}/device/driver"))
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
        if let Some(pct) = gpu1.get("usage_percent").and_then(|v| v.as_i64()) {
            assert!((0..=100).contains(&pct), "out of range: {pct}");
        }
        assert!(gpus.get("defaultGPU").is_some());
    }
}
