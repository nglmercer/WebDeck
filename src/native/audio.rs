use super::*;
mod owner;
pub(super) use owner::AudioOwner;

impl Native {
    pub(super) fn play(
        &self,
        s: &FileSource,
        volume: u64,
        device: &str,
        microphone: bool,
        local_only: bool,
        context: &Context,
    ) -> Result<Value> {
        context.check(Capability::Audio)?;
        use rodio::cpal::traits::{DeviceTrait, HostTrait};
        let host = rodio::cpal::default_host();
        let mut targets = Vec::new();
        if device.is_empty() {
            targets.push(host.default_output_device().ok_or_else(Error::execution)?);
        } else {
            targets.push(
                host.output_devices()
                    .map_err(|_| Error::execution())?
                    .find(|d| d.description().is_ok_and(|n| n.name() == device))
                    .ok_or_else(Error::execution)?,
            );
        }
        if microphone && !local_only {
            let virtual_device = host
                .output_devices()
                .map_err(|_| Error::execution())?
                .find(|d| {
                    d.description().is_ok_and(|n| {
                        let n = n.name().to_lowercase();
                        n.contains("cable input") || n.contains("webdeck")
                    })
                })
                .ok_or_else(|| {
                    Error::new(
                        ErrorCode::ExecutionFailed,
                        "Virtual microphone output not found",
                    )
                })?;
            targets.push(virtual_device);
        }
        let p = self.assets.resolve(s)?;
        self.audio.play(targets, &p, volume, context)
    }
}

pub(super) fn percent(v: &VolumeChange, current: i64) -> i64 {
    match v {
        VolumeChange::Set { percent } => *percent as i64,
        VolumeChange::Adjust { percent } => (current + percent).clamp(0, 100),
    }
}
pub(super) fn volume(v: &VolumeChange, _deadline: Instant) -> Result<Value> {
    #[cfg(target_os = "linux")]
    {
        let a = match v {
            VolumeChange::Set { percent } => format!("{percent}%"),
            VolumeChange::Adjust { percent } => {
                format!("{}{percent}%", if *percent >= 0 { "+" } else { "" })
            }
        };
        run_at(
            _deadline,
            "pactl",
            &["set-sink-volume", "@DEFAULT_SINK@", &a],
        )
    }
    #[cfg(windows)]
    {
        windows_volume(v)
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = v;
        Err(unsupported())
    }
}
pub(super) fn app_volume(app: &str, v: &VolumeChange, context: &Context) -> Result<Value> {
    #[cfg(target_os = "linux")]
    {
        let mut command = Process::new("pactl");
        command.args(["-f", "json", "list", "sink-inputs"]);
        let out = ManagedChild::output(
            command,
            context.remaining(Capability::Audio, Duration::from_secs(5))?,
        )?;
        if !out.status.success() {
            return Err(Error::execution());
        }
        let inputs: Value = serde_json::from_slice(&out.stdout).map_err(|_| Error::execution())?;
        let mut found = false;
        for i in inputs.as_array().ok_or_else(Error::execution)? {
            if i["properties"]["application.name"] == app
                || i["properties"]["application.process.binary"] == app
            {
                let id = i["index"]
                    .as_u64()
                    .ok_or_else(Error::execution)?
                    .to_string();
                let p = match v {
                    VolumeChange::Set { percent } => format!("{percent}%"),
                    VolumeChange::Adjust { percent } => {
                        format!("{}{percent}%", if *percent >= 0 { "+" } else { "" })
                    }
                };
                run_timeout(
                    "pactl",
                    &["set-sink-input-volume", &id, &p],
                    context.remaining(Capability::Audio, Duration::from_secs(5))?,
                )?;
                found = true;
            }
        }
        if found {
            Ok(json!({}))
        } else {
            Err(Error::execution())
        }
    }
    #[cfg(windows)]
    {
        windows_app_volume(app, v, context)
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = (app, v, context);
        Err(unsupported())
    }
}
pub(super) fn endpoint(name: &str, mic: bool, context: &Context) -> Result<Value> {
    context.check(Capability::Audio)?;
    #[cfg(target_os = "linux")]
    {
        let kind = if mic { "sources" } else { "sinks" };
        let mut command = Process::new("pactl");
        command.args(["-f", "json", "list", kind]);
        let out = ManagedChild::output(
            command,
            context.remaining(Capability::Audio, Duration::from_secs(5))?,
        )?;
        if !out.status.success() {
            return Err(Error::execution());
        }
        let devices: Value = serde_json::from_slice(&out.stdout).map_err(|_| Error::execution())?;
        let target = devices
            .as_array()
            .ok_or_else(Error::execution)?
            .iter()
            .find(|d| d["name"] == name || d["description"] == name)
            .and_then(|d| d["name"].as_str())
            .ok_or_else(Error::execution)?;
        return run_timeout(
            "pactl",
            &[
                if mic {
                    "set-default-source"
                } else {
                    "set-default-sink"
                },
                target,
            ],
            context.remaining(Capability::Audio, Duration::from_secs(5))?,
        );
    }
    #[cfg(windows)]
    return windows_endpoint(name, mic);
    #[allow(unreachable_code)]
    Err(unsupported())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_rejects_expired_and_denied_work_before_host_access() {
        let context = Context {
            capabilities: vec![Capability::Audio],
            deadline: Instant::now() - Duration::from_secs(1),
            depth: 0,
        };
        assert_eq!(
            endpoint("", false, &context).unwrap_err().code,
            ErrorCode::ExecutionFailed
        );
        let owner = AudioOwner::default();
        assert_eq!(
            owner
                .play(
                    Vec::new(),
                    std::path::Path::new("missing.wav"),
                    100,
                    &context
                )
                .unwrap_err()
                .code,
            ErrorCode::ExecutionFailed
        );
        let denied = Context {
            capabilities: vec![Capability::Read],
            deadline: Instant::now() + Duration::from_secs(30),
            depth: 0,
        };
        assert_eq!(
            endpoint("", true, &denied).unwrap_err().code,
            ErrorCode::Forbidden
        );
    }
}
