use super::*;
use crate::runtime::capabilities::{required_capability, CapabilityHost};
use serde::de::DeserializeOwned;

fn field<T: DeserializeOwned>(input: &Value, name: &str) -> Result<T> {
    serde_json::from_value(input.get(name).ok_or_else(Error::invalid)?.clone())
        .map_err(|_| Error::invalid())
}
#[derive(serde::Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
enum InputStep {
    #[serde(rename = "press")]
    Press { keys: Vec<String> },
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "clipboard")]
    Clipboard { text: String },
}
impl CapabilityHost for Native {
    fn call(&self, operation: &str, input: &Value, context: &Context) -> Result<Value> {
        let capability = required_capability(operation)?;
        context.check(capability)?;
        match operation {
            "network.fetch" => self.network_request(input, context),
            "input.perform" => {
                let steps: Vec<InputStep> = field(input, "steps")?;
                if steps.len() > 32 {
                    return Err(Error::invalid());
                }
                let _guard = INPUT_LOCK.try_lock().map_err(|_| {
                    Error::new(
                        ErrorCode::ExecutionFailed,
                        "Input is busy; check the host desktop permission dialog",
                    )
                })?;
                for step in steps {
                    context.check(Capability::Input)?;
                    match step {
                        InputStep::Press { keys } => {
                            if keys.is_empty()
                                || keys.len() > 16
                                || keys.iter().any(|k| k.len() > 128)
                            {
                                return Err(Error::invalid());
                            }
                            key(&keys, context.deadline)?;
                        }
                        InputStep::Text { text } => {
                            if text.len() > 65536 {
                                return Err(Error::invalid());
                            }
                            #[cfg(target_os = "linux")]
                            if input::wayland() {
                                input::text(&text, false, context.deadline)?;
                                continue;
                            }
                            agent()?.text(&text).map_err(|_| Error::execution())?;
                        }
                        InputStep::Clipboard { text } => {
                            if text.len() > 65536 {
                                return Err(Error::invalid());
                            }
                            clipboard(&text)?;
                        }
                    }
                }
                Ok(json!({}))
            }
            "window.open" => {
                open(&field::<String>(input, "target")?)?;
                Ok(json!({}))
            }
            "window.foreground" => {
                let target: String = field(input, "target")?;
                #[cfg(target_os = "linux")]
                return run_at(context.deadline, "wmctrl", &["-a", &target]);
                #[cfg(windows)]
                return windows_foreground(&target);
                #[allow(unreachable_code)]
                Err(unsupported())
            }
            "window.kill" => kill(&field::<String>(input, "target")?, context.deadline),
            "process.spawn" => {
                let executable: String = field(input, "executable")?;
                let args: Vec<String> = field(input, "args")?;
                if executable.is_empty()
                    || executable.len() > 4096
                    || args.len() > 128
                    || args.iter().any(|a| a.len() > 65536)
                {
                    return Err(Error::invalid());
                }
                let mut command = Process::new(executable);
                command.args(args);
                self.spawn(command)
            }
            "window.closeFocused" => {
                let _guard = INPUT_LOCK.try_lock().map_err(|_| Error::execution())?;
                key(&["alt".into(), "f4".into()], context.deadline)
            }
            "window.restartDesktop" => {
                #[cfg(target_os = "linux")]
                return run_at(context.deadline, "kquitapp6", &["plasmashell"]).and_then(|_| {
                    context.check(capability)?;
                    self.spawn(Process::new("plasmashell"))
                });
                #[cfg(windows)]
                return kill("explorer.exe", context.deadline).and_then(|_| {
                    context.check(capability)?;
                    self.spawn(Process::new("explorer.exe"))
                });
                #[allow(unreachable_code)]
                Err(unsupported())
            }
            "capture.pick" => color_picker(context),
            "system.exit" => {
                self.shutdown.notify_one();
                Ok(json!({}))
            }
            "system.power" => {
                let verb: String = field(input, "verb")?;
                if !["poweroff", "reboot", "suspend", "hibernate", "lock"].contains(&verb.as_str())
                {
                    return Err(Error::invalid());
                }
                power(&verb, context.deadline)
            }
            "system.screensaver" => screensaver(&field::<String>(input, "mode")?, context.deadline),
            "system.screensaverSettings" => screensaver_settings(context.deadline),
            "system.firewall" => firewall(context.deadline),
            "audio.volume" => {
                let change: VolumeChange = field(input, "change")?;
                domain::validate(
                    "VolumeChange",
                    &serde_json::to_value(&change).map_err(|_| Error::invalid())?,
                )?;
                volume(&change, context.deadline)
            }
            "audio.appVolume" => {
                let change: VolumeChange = field(input, "change")?;
                domain::validate(
                    "VolumeChange",
                    &serde_json::to_value(&change).map_err(|_| Error::invalid())?,
                )?;
                app_volume(&field::<String>(input, "application")?, &change, context)
            }
            "audio.media" => {
                let action: String = field(input, "action")?;
                let key = match action.as_str() {
                    "play-pause" => Key::MediaPlayPause,
                    "previous" => Key::MediaPrevTrack,
                    "next" => Key::MediaNextTrack,
                    "mute" => Key::VolumeMute,
                    _ => return Err(Error::invalid()),
                };
                media(&action, key, context)
            }
            "audio.endpoint" => endpoint(
                &field::<String>(input, "device")?,
                field(input, "input")?,
                context,
            ),
            "audio.stopAll" => {
                self.audio.stop();
                Ok(json!({}))
            }
            "audio.play" => self.play(
                &field(input, "source")?,
                field(input, "volume")?,
                &field::<String>(input, "outputDevice")?,
                field(input, "microphone")?,
                field(input, "localOnly")?,
                context,
            ),
            "storage.source" => {
                let source = self.source(&field(input, "source")?)?;
                if source.len() > 65536 {
                    return Err(Error::invalid());
                }
                Ok(json!(source))
            }
            "process.shell" => {
                let source: String = field(input, "source")?;
                let timeout: u64 = field(input, "timeoutMs")?;
                if source.len() > 65536 || timeout == 0 || timeout > 30000 {
                    return Err(Error::invalid());
                }
                self.shell(
                    &source,
                    context.remaining(Capability::Script, Duration::from_millis(timeout))?,
                )
            }
            "storage.button" => {
                let id: String = field(input, "id")?;
                let config = self.config.snapshot()?.config;
                let button = config
                    .layout
                    .folders
                    .iter()
                    .flat_map(|f| &f.buttons)
                    .find(|b| b.id == id)
                    .ok_or_else(Error::invalid)?;
                if let ButtonAction::Command { command } = &button.action {
                    context.check(command.capability())?;
                    serde_json::to_value(command).map_err(|_| Error::invalid())
                } else {
                    Err(Error::invalid())
                }
            }
            _ => Err(Error::invalid()),
        }
    }
    fn shutdown(&self) {
        CLIPBOARD.lock().unwrap_or_else(|p| p.into_inner()).take();
        self.processes.shutdown();
        self.audio.shutdown();
    }
}
