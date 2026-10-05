mod desktop_input;
use desktop_input::*;
mod processes;
use processes::*;
mod fake;
mod metrics;
mod network;
mod plugin_storage;
mod primitives;
mod secrets;
mod trusted;
pub use fake::FakePlatform;
mod audio;
mod websocket;
use audio::*;
mod system;
pub use metrics::usage;
pub use processes::open;
use system::*;
mod capture;
#[cfg(target_os = "linux")]
mod input;
mod integrations;
#[cfg(windows)]
mod policy_config;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use self::windows::*;
use crate::{
    contracts::*,
    domain::{self, Error, Result},
    executor::Context,
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
pub struct Platform {
    pub config: Arc<ConfigStore>,
    pub assets: Assets,
    processes: ProcessOwner,
    audio: AudioOwner,
    trusted: trusted::TrustedOwner,
    websockets: websocket::WebSocketOwner,
    pub shutdown: Arc<tokio::sync::Notify>,
}
impl Platform {
    pub fn new(
        config: Arc<ConfigStore>,
        assets: Assets,
        shutdown: Arc<tokio::sync::Notify>,
    ) -> Result<Self> {
        Ok(Self {
            config,
            assets,
            processes: ProcessOwner::default(),
            audio: AudioOwner::default(),
            trusted: trusted::TrustedOwner::default(),
            websockets: websocket::WebSocketOwner::default(),
            shutdown,
        })
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
    context.check(Capability::Window)?;
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
