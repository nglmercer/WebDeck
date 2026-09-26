//! Port of `app/buttons/usage/get_usage.py`.
//!
//! The `get_all` / `asked_devices` gating is ported 1:1; per-device readings
//! are TODO via `sysinfo` (CPU/memory, replacing `psutil`) and `nvml-wrapper`
//! (NVIDIA GPU, replacing `pynvml`/`GPUtil`).

use serde_json::{json, Value};

use crate::app::buttons::usage::asked_devices::get_asked_devices;
use crate::app::utils::{logger::log, settings::get_config::get_config};

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

    // TODO(port): sysinfo CPU/memory + nvml GPU readings with the same
    // per-device gating (`item[0]` device, `item[1]` metric).
    log().debug(&format!(
        "get_usage(get_all={get_all}, asked={asked_devices:?}): sysinfo backend not ported yet"
    ));

    let mut computer_info = json!({});
    if get_all || asked_devices.iter().any(|item| item.first().map(|s| s == "cpu").unwrap_or(false)) {
        computer_info["cpu"] = json!({"usage_percent": 0.0});
    }
    computer_info
}
