use super::*;

pub fn usage() -> Value {
    static SYSTEM: std::sync::OnceLock<Mutex<sysinfo::System>> = std::sync::OnceLock::new();
    let mut s = SYSTEM
        .get_or_init(|| Mutex::new(sysinfo::System::new_all()))
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    s.refresh_cpu_usage();
    s.refresh_memory();
    let usage = UsageSnapshot {
        memory_used: s.used_memory(),
        memory_total: s.total_memory(),
        cpu_percent: s.global_cpu_usage() as f64,
        cpus: s
            .cpus()
            .iter()
            .map(|c| CpuUsage {
                name: c.name().to_owned(),
                usage: c.cpu_usage() as f64,
            })
            .collect(),
        disks: sysinfo::Disks::new_with_refreshed_list()
            .list()
            .iter()
            .map(|d| DiskUsage {
                name: d.name().to_string_lossy().into_owned(),
                total: d.total_space(),
                available: d.available_space(),
            })
            .collect(),
        gpus: gpu_usage(),
    };
    serde_json::to_value(usage).expect("usage snapshot")
}
pub(super) fn gpu_usage() -> Vec<GpuUsage> {
    let mut gpus = Vec::new();
    if let Ok(nvml) = nvml_wrapper::Nvml::init() {
        if let Ok(count) = nvml.device_count() {
            for i in 0..count {
                if let Ok(d) = nvml.device_by_index(i) {
                    if let (Ok(name), Ok(u), Ok(m)) =
                        (d.name(), d.utilization_rates(), d.memory_info())
                    {
                        gpus.push(GpuUsage {
                            name,
                            usage: u.gpu as f64,
                            memory_used: m.used,
                            memory_total: m.total,
                        });
                    }
                }
            }
        }
    }
    #[cfg(target_os = "linux")]
    if let Ok(entries) = fs::read_dir("/sys/class/drm") {
        for e in entries.flatten() {
            let p = e.path().join("device");
            if fs::read_to_string(p.join("vendor")).is_ok_and(|s| s.trim() == "0x1002") {
                let read = |name: &str| {
                    fs::read_to_string(p.join(name))
                        .ok()
                        .and_then(|s| s.trim().parse::<u64>().ok())
                };
                if let (Some(usage), Some(used), Some(total)) = (
                    read("gpu_busy_percent"),
                    read("mem_info_vram_used"),
                    read("mem_info_vram_total"),
                ) {
                    gpus.push(GpuUsage {
                        name: e.file_name().to_string_lossy().into_owned(),
                        usage: usage as f64,
                        memory_used: used,
                        memory_total: total,
                    });
                }
            }
        }
    }
    gpus
}
