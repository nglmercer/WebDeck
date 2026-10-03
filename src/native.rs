mod desktop_input;
use desktop_input::*;
mod processes;
use processes::*;
mod metrics;
mod network;
mod primitives;
use network::*;
mod audio;
use audio::*;
mod system;
pub use metrics::usage;
pub use processes::open;
use system::*;
mod integrations;
mod scripts;
use scripts::load_plugins;
mod capture;
#[cfg(target_os = "linux")]
mod input;
#[cfg(windows)]
mod policy_config;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use self::windows::*;
use crate::{
    contracts::*,
    domain::{self, Error, Result},
    executor::{Adapter, Context},
    storage::{Assets, ConfigStore},
};
use enigo::{Direction, Enigo, Key, Keyboard, Settings as InputSettings};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    process::{Child, Command as Process},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    ast: rhai::AST,
}
pub struct Native {
    pub plugins: Vec<LoadedPlugin>,
    pub config: Arc<ConfigStore>,
    pub assets: Assets,
    processes: ProcessOwner,
    audio: AudioOwner,
    pub shutdown: Arc<tokio::sync::Notify>,
}
impl Native {
    pub fn new(
        config: Arc<ConfigStore>,
        assets: Assets,
        shutdown: Arc<tokio::sync::Notify>,
    ) -> Result<Self> {
        let plugins = load_plugins(&assets)?;
        Ok(Self {
            plugins,
            config,
            assets,
            processes: ProcessOwner::default(),
            audio: AudioOwner::default(),
            shutdown,
        })
    }
    fn exec(self: &Arc<Self>, c: &Command, x: &Context) -> Result<Value> {
        x.check(c.capability())?;
        domain::validate_command(c)?;
        use Command::*;
        // Keep clipboard-plus-paste and complete key chords atomic without
        // serializing media, network, or unrelated desktop effects.
        let _input = if matches!(
            c,
            Key { .. }
                | Write { .. }
                | Copy { .. }
                | Paste { .. }
                | Cut
                | Clipboard
                | ClearClipboard
                | SpeechRecognition
                | CloseFocused
        ) {
            Some(INPUT_LOCK.try_lock().map_err(|_| {
                Error::new(
                    ErrorCode::ExecutionFailed,
                    "Input is busy; check the host desktop permission dialog",
                )
            })?)
        } else {
            None
        };
        match c {
            Command::Button { button_id } => {
                let config = self.config.snapshot()?.config;
                let command = config
                    .layout
                    .folders
                    .iter()
                    .flat_map(|f| &f.buttons)
                    .find(|b| &b.id == button_id)
                    .and_then(|b| {
                        if let ButtonAction::Command { command } = &b.action {
                            Some(command.clone())
                        } else {
                            None
                        }
                    })
                    .ok_or_else(Error::invalid)?;
                self.exec(&command, &x.nested())
            }
            Debug { data } => Ok(json!({"data":data})),
            Exit => {
                self.shutdown.notify_one();
                Ok(json!({}))
            }
            Usage => Ok(usage()),
            Key { keys } => key(keys, x.deadline),
            Write { text, send } => {
                #[cfg(target_os = "linux")]
                if input::wayland() {
                    return input::text(text, *send, x.deadline);
                }
                let mut a = agent()?;
                x.check(Capability::Input)?;
                a.text(text).map_err(|_| Error::execution())?;
                if *send {
                    x.check(Capability::Input)?;
                    a.key(enigo::Key::Return, Direction::Click)
                        .map_err(|_| Error::execution())?;
                }
                Ok(json!({}))
            }
            Copy {
                text,
                use_selection,
            } => {
                if *use_selection {
                    key(&["ctrl".into(), "c".into()], x.deadline)
                } else {
                    clipboard(text)
                }
            }
            Paste {
                text,
                use_selection,
            } => {
                if !use_selection {
                    clipboard(text)?;
                }
                key(&["ctrl".into(), "v".into()], x.deadline)
            }
            Cut => key(&["ctrl".into(), "x".into()], x.deadline),
            ClearClipboard => clipboard(""),
            Clipboard => key(&["meta".into(), "v".into()], x.deadline),
            SpeechRecognition => key(&["meta".into(), "h".into()], x.deadline),
            Open { target } => {
                open(target)?;
                Ok(json!({}))
            }
            Foreground { target } => {
                #[cfg(target_os = "linux")]
                return run_at(x.deadline, "wmctrl", &["-a", target]);
                #[cfg(windows)]
                return windows_foreground(target);
                #[allow(unreachable_code)]
                Err(unsupported())
            }
            CloseFocused => key(&["alt".into(), "f4".into()], x.deadline),
            Kill { target } => kill(target, x.deadline),
            Restart { target } => {
                kill(target, x.deadline)?;
                x.check(c.capability())?;
                self.spawn(Process::new(target))
            }
            RestartDesktop => {
                #[cfg(target_os = "linux")]
                return run_at(x.deadline, "kquitapp6", &["plasmashell"]).and_then(|_| {
                    x.check(c.capability())?;
                    self.spawn(Process::new("plasmashell"))
                });
                #[cfg(windows)]
                return kill("explorer.exe", x.deadline).and_then(|_| {
                    x.check(c.capability())?;
                    self.spawn(Process::new("explorer.exe"))
                });
                #[allow(unreachable_code)]
                Err(unsupported())
            }
            PlayPause => media("play-pause", enigo::Key::MediaPlayPause, x),
            Previous => media("previous", enigo::Key::MediaPrevTrack, x),
            Next => media("next", enigo::Key::MediaNextTrack, x),
            Mute => media("mute", enigo::Key::VolumeMute, x),
            Volume { change } => volume(change, x.deadline),
            AppVolume {
                application,
                change,
            } => app_volume(application, change, x),
            Microphone { device } => endpoint(device, true, x),
            Speakers { device } => endpoint(device, false, x),
            Shutdown => power("poweroff", x.deadline),
            Reboot => power("reboot", x.deadline),
            Sleep => power("suspend", x.deadline),
            Hibernate => power("hibernate", x.deadline),
            Lock => power("lock", x.deadline),
            ScreensaverSettings => screensaver_settings(x.deadline),
            Screensaver { mode } => screensaver(mode, x.deadline),
            Firewall => firewall(x.deadline),
            StopSound => {
                self.audio.stop();
                Ok(json!({}))
            }
            PlaySound {
                source,
                volume,
                output_device,
                microphone,
                local_only,
            } => self.play(source, *volume, output_device, *microphone, *local_only, x),
            Fetch {
                method,
                url,
                headers,
                body,
                timeout_seconds,
            } => fetch(
                method,
                url,
                headers,
                body,
                x.remaining(c.capability(), Duration::from_secs(*timeout_seconds))?,
            ),
            Script { source } => self.script(source, x),
            Shell {
                source,
                timeout_seconds,
            } => self.shell(
                &self.source(source)?,
                x.remaining(c.capability(), Duration::from_secs(*timeout_seconds))?,
            ),
            Obs { action, target } => self.obs(action, target, x),
            Spotify {
                action,
                target,
                change,
            } => self.spotify(action, target, change, x),
            Plugin {
                plugin_id,
                version,
                action_id,
                args,
            } => self.plugin(plugin_id, version, action_id, args, x),
            ColorPicker => color_picker(x),
        }
    }
    fn source(&self, s: &ScriptSource) -> Result<String> {
        match s {
            ScriptSource::Inline { code } => Ok(code.clone()),
            ScriptSource::File { source } => read_script_source(
                fs::File::open(self.assets.resolve(source)?).map_err(|_| Error::execution())?,
            ),
        }
    }
    fn spawn(&self, command: Process) -> Result<Value> {
        self.processes.spawn(command)
    }
    fn shell(&self, source: &str, timeout: Duration) -> Result<Value> {
        #[cfg(windows)]
        let mut command = Process::new("cmd");
        #[cfg(windows)]
        command.args(["/C", source]);
        #[cfg(not(windows))]
        let mut command = Process::new("sh");
        #[cfg(not(windows))]
        command.args(["-c", source]);
        let status = ManagedChild::spawn(command)?.wait(timeout)?;
        if status.success() {
            Ok(json!({"exit_code":status.code()}))
        } else {
            Err(Error::execution())
        }
    }
}

fn read_script_source(reader: impl Read) -> Result<String> {
    let mut bytes = Vec::new();
    reader
        .take(65537)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::execution())?;
    if bytes.len() > 65536 {
        return Err(Error::invalid());
    }
    String::from_utf8(bytes).map_err(|_| Error::invalid())
}

impl Adapter for Arc<Native> {
    fn execute(&self, c: &Command, x: &Context) -> Result<Value> {
        self.exec(c, x)
    }
    fn shutdown(&self) {
        CLIPBOARD.lock().unwrap_or_else(|p| p.into_inner()).take();
        self.processes.shutdown();
        self.audio.shutdown();
    }
}
fn unsupported() -> Error {
    Error::new(
        ErrorCode::UnsupportedPlatform,
        "This action is unavailable on this platform",
    )
}
fn media(action: &str, k: Key, context: &Context) -> Result<Value> {
    #[cfg(target_os = "linux")]
    {
        if action == "mute" {
            return run_timeout(
                "pactl",
                &["set-sink-mute", "@DEFAULT_SINK@", "toggle"],
                context.remaining(Capability::Audio, Duration::from_secs(5))?,
            );
        }
        let _ = k;
        integrations::media(action, context)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = action;
        context.check(Capability::Audio)?;
        agent()?
            .key(k, Direction::Click)
            .map_err(|_| Error::execution())?;
        Ok(json!({}))
    }
}
fn color_picker(context: &Context) -> Result<Value> {
    let c = capture::get_mouse_pixel_color(context.deadline).map_err(|_| Error::execution())?;
    context.check(Command::ColorPicker.capability())?;
    clipboard(&c.hex)?;
    Ok(json!({"hex":c.hex,"rgb":c.rgb,"hsl":c.hsl}))
}

#[cfg(test)]
mod source_tests {
    use super::*;

    #[test]
    fn source_accepts_limit_and_rejects_invalid_utf8() {
        assert_eq!(
            read_script_source(&vec![b'a'; 65536][..]).unwrap().len(),
            65536
        );
        assert!(read_script_source(&[0xff][..]).is_err());
    }

    #[test]
    fn oversized_source_reads_only_limit_plus_one() {
        let mut input = std::io::Cursor::new(vec![b'a'; 1024 * 1024]);
        assert!(read_script_source(&mut input).is_err());
        assert_eq!(input.position(), 65537);
    }
}
