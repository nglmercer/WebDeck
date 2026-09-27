//! Disk section of `get_usage` (extracted from `get_usage.rs`).
//!
//! Disk enumeration, eval-safe aliases, and per-disk metric gating live
//! here; the `get_all` merge tail stays in [`super::get_usage`].

use serde_json::{json, Value};
use sysinfo::Disks;

use super::get_usage::{asked_metric, round1, round2};

/// Linux-only eval-safe disk alias: device basename with every
/// non-identifier char mapped to `_` (`/dev/nvme0n1p4` → `nvme0n1p4`),
/// so usage tiles can address disks through the JS `eval` paths.
#[cfg(target_os = "linux")]
pub(crate) fn disk_alias_key(disk_name: &str) -> String {
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

/// Response keys for one disk, in insert order: the eval-safe basename
/// alias (Linux only; elsewhere `alias == disk_name` so it collapses),
/// the Linux-only `"C"` alias when this disk is the root filesystem
/// (`C:` is the Windows system drive, so Windows-authored "C disk" tiles
/// keep working on Linux), then the 1:1 key last so it wins collisions.
fn disk_response_keys(disk_name: &str, alias: &str, mount_point: &std::path::Path) -> Vec<String> {
    let mut keys = Vec::with_capacity(3);
    if alias != disk_name {
        keys.push(alias.to_string());
    }
    if mount_point == std::path::Path::new("/") {
        keys.push("C".to_string());
    }
    keys.push(disk_name.to_string());
    keys
}

/// Port of the hard-disk branch of `get_usage`.
pub(crate) fn collect_disks(get_all: bool, asked_devices: &[Vec<String>]) -> Value {
    let mut disks_map = serde_json::Map::new();
    for disk in Disks::new_with_refreshed_list().iter() {
        // Per-disk errors are swallowed like Python's try/except/pass.
        let raw_name = disk.name().to_string_lossy();
        let disk_name = raw_name.replace('\\', "").replace(':', "");
        // Linux-only: device paths are not valid JS eval paths, so the
        // basename alias is exposed (and matchable) alongside the 1:1 key.
        #[cfg(target_os = "linux")]
        let alias = disk_alias_key(&disk_name);
        #[cfg(not(target_os = "linux"))]
        let alias = disk_name.clone();
        let keys = disk_response_keys(&disk_name, &alias, disk.mount_point());
        let wanted = get_all || keys.iter().any(|k| asked_metric(asked_devices, 1, k));
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
        for key in &keys {
            disks_map.insert(key.clone(), value.clone());
        }
    }
    Value::Object(disks_map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn disk_alias_is_eval_safe() {
        assert_eq!(disk_alias_key("/dev/nvme0n1p4"), "nvme0n1p4");
        assert_eq!(disk_alias_key("/dev/mapper/vg-lv"), "vg_lv");
        assert_eq!(disk_alias_key("C"), "C");
        assert_eq!(disk_alias_key("/"), "disk");
        assert_eq!(disk_alias_key("9lives"), "_9lives");
    }

    #[test]
    fn disk_keys_expose_root_as_c() {
        let keys = disk_response_keys("/dev/nvme0n1p4", "nvme0n1p4", std::path::Path::new("/"));
        assert_eq!(keys, vec!["nvme0n1p4", "C", "/dev/nvme0n1p4"]);
        // Non-root mounts get no "C" alias.
        let keys = disk_response_keys("/dev/sda1", "sda1", std::path::Path::new("/mnt/data"));
        assert_eq!(keys, vec!["sda1", "/dev/sda1"]);
        // 1:1 keys without alias stay untouched (Windows shape).
        let keys = disk_response_keys("C", "C", std::path::Path::new("C:\\"));
        assert_eq!(keys, vec!["C"]);
    }
}
