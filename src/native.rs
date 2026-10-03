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
    children: Mutex<Vec<Child>>,
    sounds: Mutex<Vec<(rodio::MixerDeviceSink, rodio::Player)>>,
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
            children: Mutex::new(Vec::new()),
            sounds: Mutex::new(Vec::new()),
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
            Key { keys } => key(keys),
            Write { text, send } => {
                #[cfg(target_os = "linux")]
                if input::wayland() {
                    return input::text(text, *send);
                }
                let mut a = agent()?;
                a.text(text).map_err(|_| Error::execution())?;
                if *send {
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
                    key(&["ctrl".into(), "c".into()])
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
                key(&["ctrl".into(), "v".into()])
            }
            Cut => key(&["ctrl".into(), "x".into()]),
            ClearClipboard => clipboard(""),
            Clipboard => key(&["meta".into(), "v".into()]),
            SpeechRecognition => key(&["meta".into(), "h".into()]),
            Open { target } => {
                open(target)?;
                Ok(json!({}))
            }
            Foreground { target } => {
                #[cfg(target_os = "linux")]
                return run("wmctrl", &["-a", target]);
                #[cfg(windows)]
                return windows_foreground(target);
                #[allow(unreachable_code)]
                Err(unsupported())
            }
            CloseFocused => key(&["alt".into(), "f4".into()]),
            Kill { target } => kill(target),
            Restart { target } => {
                kill(target)?;
                self.spawn(Process::new(target))
            }
            RestartDesktop => {
                #[cfg(target_os = "linux")]
                return run("kquitapp6", &["plasmashell"])
                    .and_then(|_| self.spawn(Process::new("plasmashell")));
                #[cfg(windows)]
                return kill("explorer.exe").and_then(|_| self.spawn(Process::new("explorer.exe")));
                #[allow(unreachable_code)]
                Err(unsupported())
            }
            PlayPause => media("play-pause", enigo::Key::MediaPlayPause),
            Previous => media("previous", enigo::Key::MediaPrevTrack),
            Next => media("next", enigo::Key::MediaNextTrack),
            Mute => media("mute", enigo::Key::VolumeMute),
            Volume { change } => volume(change),
            AppVolume {
                application,
                change,
            } => app_volume(application, change),
            Microphone { device } => endpoint(device, true),
            Speakers { device } => endpoint(device, false),
            Shutdown => power("poweroff"),
            Reboot => power("reboot"),
            Sleep => power("suspend"),
            Hibernate => power("hibernate"),
            Lock => power("lock"),
            ScreensaverSettings => screensaver_settings(),
            Screensaver { mode } => screensaver(mode),
            Firewall => firewall(),
            StopSound => {
                for (_, p) in self
                    .sounds
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .drain(..)
                {
                    p.stop();
                }
                Ok(json!({}))
            }
            PlaySound {
                source,
                volume,
                output_device,
                microphone,
                local_only,
            } => self.play(source, *volume, output_device, *microphone, *local_only),
            Fetch {
                method,
                url,
                headers,
                body,
                timeout_seconds,
            } => fetch(method, url, headers, body, *timeout_seconds),
            Script { source } => self.script(source, x),
            Shell {
                source,
                timeout_seconds,
            } => self.shell(
                &self.source(source)?,
                Duration::from_secs(*timeout_seconds)
                    .min(x.deadline.saturating_duration_since(Instant::now())),
            ),
            Obs { action, target } => self.obs(action, target),
            Spotify {
                action,
                target,
                change,
            } => self.spotify(action, target, change),
            Plugin {
                plugin_id,
                version,
                action_id,
                args,
            } => self.plugin(plugin_id, version, action_id, args, x),
            ColorPicker => color_picker(),
        }
    }
    fn source(&self, s: &ScriptSource) -> Result<String> {
        match s {
            ScriptSource::Inline { code } => Ok(code.clone()),
            ScriptSource::File { source } => {
                let b = fs::read(self.assets.resolve(source)?).map_err(|_| Error::execution())?;
                if b.len() > 65536 {
                    return Err(Error::invalid());
                }
                String::from_utf8(b).map_err(|_| Error::invalid())
            }
        }
    }
    fn spawn(&self, mut p: Process) -> Result<Value> {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            p.process_group(0);
        }
        let mut c = self.children.lock().unwrap_or_else(|p| p.into_inner());
        c.retain_mut(|c| !matches!(c.try_wait(), Ok(Some(_))));
        if c.len() >= 64 {
            return Err(Error::new(
                ErrorCode::CapacityExhausted,
                "Process capacity exhausted",
            ));
        }
        c.push(p.spawn().map_err(|_| Error::execution())?);
        Ok(json!({"launched":true}))
    }
    fn shell(&self, source: &str, timeout: Duration) -> Result<Value> {
        #[cfg(windows)]
        let mut p = {
            let mut p = Process::new("cmd");
            p.args(["/C", source]);
            p
        };
        #[cfg(not(windows))]
        let mut p = {
            let mut p = Process::new("sh");
            p.args(["-c", source]);
            p
        };
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            p.process_group(0);
        }
        let mut child = p.spawn().map_err(|_| Error::execution())?;
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(s) = child.try_wait().map_err(|_| Error::execution())? {
                stop(&mut child);
                return if s.success() {
                    Ok(json!({"exit_code":s.code()}))
                } else {
                    Err(Error::execution())
                };
            }
            if Instant::now() >= deadline {
                stop(&mut child);
                return Err(Error::execution());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Adapter for Arc<Native> {
    fn execute(&self, c: &Command, x: &Context) -> Result<Value> {
        self.exec(c, x)
    }
    fn shutdown(&self) {
        CLIPBOARD.lock().unwrap_or_else(|p| p.into_inner()).take();
        for child in self
            .children
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .drain(..)
        {
            let mut c = child;
            stop(&mut c);
        }
        for (_, p) in self
            .sounds
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .drain(..)
        {
            p.stop();
        }
    }
}
fn stop(c: &mut Child) {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(c.id() as i32), libc::SIGKILL);
    }
    #[cfg(windows)]
    {
        let _ = Process::new("taskkill")
            .args(["/PID", &c.id().to_string(), "/T", "/F"])
            .status();
    }
    let _ = c.kill();
    let _ = c.wait();
}
fn unsupported() -> Error {
    Error::new(
        ErrorCode::UnsupportedPlatform,
        "This action is unavailable on this platform",
    )
}
fn agent() -> Result<Enigo> {
    Enigo::new(&InputSettings::default()).map_err(|_| Error::execution())
}
fn map_key(s: &str) -> Result<Key> {
    Ok(match s.to_lowercase().as_str() {
        "ctrl" | "control" => Key::Control,
        "shift" => Key::Shift,
        "alt" => Key::Alt,
        "meta" | "super" | "win" | "windows" => Key::Meta,
        "enter" | "return" => Key::Return,
        "escape" | "esc" => Key::Escape,
        "tab" => Key::Tab,
        "space" => Key::Space,
        "backspace" => Key::Backspace,
        "delete" => Key::Delete,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        "up" => Key::UpArrow,
        "down" => Key::DownArrow,
        "left" => Key::LeftArrow,
        "right" => Key::RightArrow,
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,
        _ => {
            if s.chars().count() != 1 {
                return Err(Error::invalid());
            }
            Key::Unicode(s.chars().next().ok_or_else(Error::invalid)?)
        }
    })
}
static INPUT_LOCK: Mutex<()> = Mutex::new(());
fn key(s: &[String]) -> Result<Value> {
    #[cfg(target_os = "linux")]
    if input::wayland() {
        return input::keys(s);
    }
    let keys: Vec<_> = s.iter().map(|s| map_key(s)).collect::<Result<_>>()?;
    let mut a = agent()?;
    let mut pressed = Vec::new();
    let result = (|| {
        for k in keys {
            a.key(k, Direction::Press).map_err(|_| Error::execution())?;
            pressed.push(k);
        }
        Ok(json!({}))
    })();
    for k in pressed.into_iter().rev() {
        let _ = a.key(k, Direction::Release);
    }
    result
}
static CLIPBOARD: Mutex<Option<arboard::Clipboard>> = Mutex::new(None);
fn clipboard(s: &str) -> Result<Value> {
    let mut slot = CLIPBOARD.lock().unwrap_or_else(|p| p.into_inner());
    if slot.is_none() {
        *slot = Some(arboard::Clipboard::new().map_err(|_| {
            Error::new(
                ErrorCode::ExecutionFailed,
                "Cannot connect to the host clipboard",
            )
        })?);
    }
    slot.as_mut()
        .expect("initialized clipboard")
        .set_text(s.to_owned())
        .map_err(|_| Error::execution())?;
    Ok(json!({}))
}
pub fn open(s: &str) -> Result<()> {
    #[cfg(windows)]
    let status = Process::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", s])
        .spawn();
    #[cfg(not(windows))]
    let status = Process::new("xdg-open").arg(s).spawn();
    status.map_err(|_| Error::execution())?;
    Ok(())
}
fn run(program: &str, args: &[&str]) -> Result<Value> {
    let mut p = Process::new(program);
    p.args(args);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        p.process_group(0);
    }
    let mut child = p.spawn().map_err(|e| {
        Error::new(
            ErrorCode::ExecutionFailed,
            format!("Cannot launch {program}: {e}"),
        )
    })?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().map_err(|_| Error::execution())? {
            stop(&mut child);
            return if status.success() {
                Ok(json!({}))
            } else {
                Err(Error::new(
                    ErrorCode::ExecutionFailed,
                    format!("{program} exited with {status}"),
                ))
            };
        }
        if Instant::now() >= deadline {
            stop(&mut child);
            return Err(Error::new(
                ErrorCode::ExecutionFailed,
                format!("{program} timed out"),
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn kill(s: &str) -> Result<Value> {
    #[cfg(windows)]
    return run("taskkill", &["/F", "/IM", s]);
    #[cfg(not(windows))]
    return run("pkill", &["-x", "--", s]);
}
fn media(action: &str, k: Key) -> Result<Value> {
    #[cfg(target_os = "linux")]
    {
        if action == "mute" {
            return run("pactl", &["set-sink-mute", "@DEFAULT_SINK@", "toggle"]);
        }
        let _ = k;
        integrations::media(action)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = action;
        agent()?
            .key(k, Direction::Click)
            .map_err(|_| Error::execution())?;
        Ok(json!({}))
    }
}
fn power(action: &str) -> Result<Value> {
    #[cfg(target_os = "linux")]
    return if action == "lock" {
        run("loginctl", &["lock-session"])
    } else {
        run("systemctl", &[action])
    };
    #[cfg(windows)]
    return match action {
        "poweroff" => run("shutdown", &["/s", "/f", "/t", "0"]),
        "reboot" => run("shutdown", &["/r", "/f", "/t", "0"]),
        "suspend" => run("rundll32.exe", &["powrprof.dll,SetSuspendState", "0,1,0"]),
        "hibernate" => run("shutdown", &["/h"]),
        "lock" => run("rundll32.exe", &["user32.dll,LockWorkStation"]),
        _ => Err(unsupported()),
    };
    #[allow(unreachable_code)]
    Err(unsupported())
}
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
fn gpu_usage() -> Vec<GpuUsage> {
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

fn fetch(
    method: &str,
    url: &str,
    headers: &BTreeMap<String, String>,
    body: &str,
    t: u64,
) -> Result<Value> {
    let c = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(t))
        .build()
        .map_err(|_| Error::execution())?;
    let method = reqwest::Method::from_bytes(method.as_bytes()).map_err(|_| Error::invalid())?;
    let mut r = c.request(method, url).body(body.to_owned());
    for (k, v) in headers {
        r = r.header(k, v);
    }
    let r = r.send().map_err(|_| Error::execution())?;
    let status = r.status();
    let mut b = Vec::new();
    r.take(65537)
        .read_to_end(&mut b)
        .map_err(|_| Error::execution())?;
    if !status.is_success() {
        return Err(Error::execution());
    }
    let truncated = b.len() > 65536;
    b.truncate(65536);
    Ok(json!({"status":status.as_u16(),"body":String::from_utf8_lossy(&b),"truncated":truncated}))
}
impl Native {
    fn play(
        &self,
        s: &FileSource,
        volume: u64,
        device: &str,
        microphone: bool,
        local_only: bool,
    ) -> Result<Value> {
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
        let mut sounds = self.sounds.lock().unwrap_or_else(|p| p.into_inner());
        sounds.retain(|(_, p)| !p.empty());
        if sounds.len() + targets.len() > 32 {
            return Err(Error::new(
                ErrorCode::CapacityExhausted,
                "Sound capacity exhausted",
            ));
        }
        for d in targets {
            let sink = rodio::DeviceSinkBuilder::from_device(d)
                .map_err(|_| Error::execution())?
                .open_stream()
                .map_err(|_| Error::execution())?;
            let player = rodio::Player::connect_new(sink.mixer());
            let source =
                rodio::Decoder::try_from(fs::File::open(&p).map_err(|_| Error::execution())?)
                    .map_err(|_| Error::execution())?;
            player.set_volume(volume as f32 / 100.0);
            player.append(source);
            sounds.push((sink, player));
        }
        Ok(json!({"playing":true}))
    }
}

fn percent(v: &VolumeChange, current: i64) -> i64 {
    match v {
        VolumeChange::Set { percent } => *percent as i64,
        VolumeChange::Adjust { percent } => (current + percent).clamp(0, 100),
    }
}
fn volume(v: &VolumeChange) -> Result<Value> {
    #[cfg(target_os = "linux")]
    {
        let a = match v {
            VolumeChange::Set { percent } => format!("{percent}%"),
            VolumeChange::Adjust { percent } => {
                format!("{}{percent}%", if *percent >= 0 { "+" } else { "" })
            }
        };
        run("pactl", &["set-sink-volume", "@DEFAULT_SINK@", &a])
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
fn app_volume(app: &str, v: &VolumeChange) -> Result<Value> {
    #[cfg(target_os = "linux")]
    {
        let out = Process::new("pactl")
            .args(["-f", "json", "list", "sink-inputs"])
            .output()
            .map_err(|_| Error::execution())?;
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
                run("pactl", &["set-sink-input-volume", &id, &p])?;
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
        windows_app_volume(app, v)
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = (app, v);
        Err(unsupported())
    }
}
fn endpoint(name: &str, mic: bool) -> Result<Value> {
    #[cfg(target_os = "linux")]
    {
        let kind = if mic { "sources" } else { "sinks" };
        let out = Process::new("pactl")
            .args(["-f", "json", "list", kind])
            .output()
            .map_err(|_| Error::execution())?;
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
        return run(
            "pactl",
            &[
                if mic {
                    "set-default-source"
                } else {
                    "set-default-sink"
                },
                target,
            ],
        );
    }
    #[cfg(windows)]
    return windows_endpoint(name, mic);
    #[allow(unreachable_code)]
    Err(unsupported())
}
fn screensaver_settings() -> Result<Value> {
    #[cfg(windows)]
    return run(
        "rundll32.exe",
        &["desk.cpl,InstallScreenSaver", "toasters.scr"],
    );
    #[cfg(target_os = "linux")]
    return run("systemsettings", &["kcm_screenlocker"]);
    #[allow(unreachable_code)]
    Err(unsupported())
}
fn screensaver(mode: &str) -> Result<Value> {
    #[cfg(target_os = "linux")]
    return match mode {
        "display_off" => run("xset", &["dpms", "force", "off"]),
        "start" => run("loginctl", &["lock-session"]),
        "stop" => run(
            "dbus-send",
            &[
                "--session",
                "--dest=org.freedesktop.ScreenSaver",
                "/ScreenSaver",
                "org.freedesktop.ScreenSaver.SimulateUserActivity",
            ],
        ),
        _ => Err(Error::invalid()),
    };
    #[cfg(windows)]
    return match mode {
        "start" => run("scrnsave.scr", &["/s"]),
        "stop" => key(&["ctrl".into()]),
        "display_off" => windows_display_off(),
        _ => Err(Error::invalid()),
    };
    #[allow(unreachable_code)]
    Err(unsupported())
}
fn firewall() -> Result<Value> {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().map_err(|_| Error::execution())?;
        run(
            "netsh",
            &[
                "advfirewall",
                "firewall",
                "add",
                "rule",
                "name=WebDeck v2",
                "dir=in",
                "action=allow",
                &format!("program={}", exe.display()),
                "enable=yes",
            ],
        )
    }
    #[cfg(not(windows))]
    Err(unsupported())
}
fn color_picker() -> Result<Value> {
    let c = capture::get_mouse_pixel_color().map_err(|_| Error::execution())?;
    clipboard(&c.hex)?;
    Ok(json!({"hex":c.hex,"rgb":c.rgb,"hsl":c.hsl}))
}
