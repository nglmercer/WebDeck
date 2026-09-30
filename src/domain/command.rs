//! Pure legacy parsing: prefix precedence is compatibility behavior, not
//! whitespace tokenization. Raw placeholders/quoted paths survive intact.
use super::error::{AppError, ErrorCode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Read,
    Input,
    Audio,
    Window,
    Power,
    Script,
    Network,
    Plugin,
    Admin,
    Settings,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Resource {
    Read,
    Input,
    Audio,
    Window,
    Power,
    Script,
    Spotify,
    Obs,
    Fetch,
    Plugin,
    Admin,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandKind {
    Debug,
    Exit,
    Usage,
    StopSound,
    PlaySound,
    Shutdown,
    Reboot,
    Sleep,
    Hibernate,
    Lock,
    ScreensaverSettings,
    Screensaver,
    Key,
    RestartDesktop,
    Kill,
    Restart,
    ClearClipboard,
    Write,
    WriteAndSend,
    AppVolume,
    Mute,
    PlayPause,
    Previous,
    Next,
    SpeechRecognition,
    CloseFocused,
    Foreground,
    Microphone,
    Speakers,
    Copy,
    Paste,
    Cut,
    Clipboard,
    Volume,
    Spotify,
    Obs,
    ColorPicker,
    Open,
    Exec,
    Batch,
    Fetch,
    BypassFirewall,
    Plugin,
    Unknown,
}
#[derive(Debug, Clone)]
pub struct ParsedCommand {
    pub kind: CommandKind,
    pub original: String,
    pub normalized: String,
    pub arguments: String,
    pub capability: Capability,
    pub resource: Resource,
}
#[derive(Debug, Clone, Serialize)]
pub struct CommandDescriptor {
    pub kind: CommandKind,
    pub prefixes: &'static [&'static str],
    pub capability: Capability,
    #[serde(skip)]
    pub resource: Resource,
}
pub const BUILTINS: &[CommandDescriptor] = &[
    CommandDescriptor {
        kind: CommandKind::Debug,
        prefixes: &["/debug-send"],
        capability: Capability::Read,
        resource: Resource::Read,
    },
    CommandDescriptor {
        kind: CommandKind::Exit,
        prefixes: &["/exit"],
        capability: Capability::Power,
        resource: Resource::Power,
    },
    CommandDescriptor {
        kind: CommandKind::Usage,
        prefixes: &["/usage"],
        capability: Capability::Read,
        resource: Resource::Read,
    },
    CommandDescriptor {
        kind: CommandKind::StopSound,
        prefixes: &["/stop_sound"],
        capability: Capability::Audio,
        resource: Resource::Audio,
    },
    CommandDescriptor {
        kind: CommandKind::PlaySound,
        prefixes: &["/playsound ", "/playlocalsound "],
        capability: Capability::Audio,
        resource: Resource::Audio,
    },
    CommandDescriptor {
        kind: CommandKind::Shutdown,
        prefixes: &["/PCshutdown"],
        capability: Capability::Power,
        resource: Resource::Power,
    },
    CommandDescriptor {
        kind: CommandKind::Reboot,
        prefixes: &["/PCrestart"],
        capability: Capability::Power,
        resource: Resource::Power,
    },
    CommandDescriptor {
        kind: CommandKind::Sleep,
        prefixes: &["/PCsleep"],
        capability: Capability::Power,
        resource: Resource::Power,
    },
    CommandDescriptor {
        kind: CommandKind::Hibernate,
        prefixes: &["/PChibernate"],
        capability: Capability::Power,
        resource: Resource::Power,
    },
    CommandDescriptor {
        kind: CommandKind::Lock,
        prefixes: &["/locksession"],
        capability: Capability::Power,
        resource: Resource::Power,
    },
    CommandDescriptor {
        kind: CommandKind::ScreensaverSettings,
        prefixes: &["/screensaversettings"],
        capability: Capability::Power,
        resource: Resource::Power,
    },
    CommandDescriptor {
        kind: CommandKind::Screensaver,
        prefixes: &["/screensaver"],
        capability: Capability::Power,
        resource: Resource::Power,
    },
    CommandDescriptor {
        kind: CommandKind::Key,
        prefixes: &["/key"],
        capability: Capability::Input,
        resource: Resource::Input,
    },
    CommandDescriptor {
        kind: CommandKind::RestartDesktop,
        prefixes: &["/restartexplorer"],
        capability: Capability::Window,
        resource: Resource::Window,
    },
    CommandDescriptor {
        kind: CommandKind::Kill,
        prefixes: &["/kill", "/taskill", "/taskkill", "/forceclose"],
        capability: Capability::Window,
        resource: Resource::Window,
    },
    CommandDescriptor {
        kind: CommandKind::Restart,
        prefixes: &["/restart"],
        capability: Capability::Window,
        resource: Resource::Window,
    },
    CommandDescriptor {
        kind: CommandKind::ClearClipboard,
        prefixes: &["/clearclipboard"],
        capability: Capability::Input,
        resource: Resource::Input,
    },
    CommandDescriptor {
        kind: CommandKind::Write,
        prefixes: &["/write "],
        capability: Capability::Input,
        resource: Resource::Input,
    },
    CommandDescriptor {
        kind: CommandKind::WriteAndSend,
        prefixes: &["/writeandsend "],
        capability: Capability::Input,
        resource: Resource::Input,
    },
    CommandDescriptor {
        kind: CommandKind::AppVolume,
        prefixes: &["/appvolume +", "/appvolume -", "/appvolume set"],
        capability: Capability::Audio,
        resource: Resource::Audio,
    },
    CommandDescriptor {
        kind: CommandKind::Mute,
        prefixes: &["/soundcontrol mute"],
        capability: Capability::Audio,
        resource: Resource::Audio,
    },
    CommandDescriptor {
        kind: CommandKind::PlayPause,
        prefixes: &["/mediacontrol playpause"],
        capability: Capability::Audio,
        resource: Resource::Audio,
    },
    CommandDescriptor {
        kind: CommandKind::Previous,
        prefixes: &["/mediacontrol previous"],
        capability: Capability::Audio,
        resource: Resource::Audio,
    },
    CommandDescriptor {
        kind: CommandKind::Next,
        prefixes: &["/mediacontrol next"],
        capability: Capability::Audio,
        resource: Resource::Audio,
    },
    CommandDescriptor {
        kind: CommandKind::SpeechRecognition,
        prefixes: &["/speechrecognition"],
        capability: Capability::Input,
        resource: Resource::Input,
    },
    CommandDescriptor {
        kind: CommandKind::CloseFocused,
        prefixes: &["/superAltF4"],
        capability: Capability::Window,
        resource: Resource::Window,
    },
    CommandDescriptor {
        kind: CommandKind::Foreground,
        prefixes: &["/firstplan"],
        capability: Capability::Window,
        resource: Resource::Window,
    },
    CommandDescriptor {
        kind: CommandKind::Microphone,
        prefixes: &["/setmicrophone"],
        capability: Capability::Audio,
        resource: Resource::Audio,
    },
    CommandDescriptor {
        kind: CommandKind::Speakers,
        prefixes: &["/setoutputdevice"],
        capability: Capability::Audio,
        resource: Resource::Audio,
    },
    CommandDescriptor {
        kind: CommandKind::Copy,
        prefixes: &["/copy"],
        capability: Capability::Input,
        resource: Resource::Input,
    },
    CommandDescriptor {
        kind: CommandKind::Paste,
        prefixes: &["/paste"],
        capability: Capability::Input,
        resource: Resource::Input,
    },
    CommandDescriptor {
        kind: CommandKind::Cut,
        prefixes: &["/cut"],
        capability: Capability::Input,
        resource: Resource::Input,
    },
    CommandDescriptor {
        kind: CommandKind::Clipboard,
        prefixes: &["/clipboard"],
        capability: Capability::Input,
        resource: Resource::Input,
    },
    CommandDescriptor {
        kind: CommandKind::Volume,
        prefixes: &["/volume"],
        capability: Capability::Audio,
        resource: Resource::Audio,
    },
    CommandDescriptor {
        kind: CommandKind::Spotify,
        prefixes: &["/spotify"],
        capability: Capability::Network,
        resource: Resource::Spotify,
    },
    CommandDescriptor {
        kind: CommandKind::Obs,
        prefixes: &["/obs"],
        capability: Capability::Network,
        resource: Resource::Obs,
    },
    CommandDescriptor {
        kind: CommandKind::ColorPicker,
        prefixes: &["/colorpicker"],
        capability: Capability::Window,
        resource: Resource::Window,
    },
    CommandDescriptor {
        kind: CommandKind::Open,
        prefixes: &["/openfolder", "/opendir", "/openfile", "/start"],
        capability: Capability::Window,
        resource: Resource::Window,
    },
    CommandDescriptor {
        kind: CommandKind::Exec,
        prefixes: &["/exec"],
        capability: Capability::Script,
        resource: Resource::Script,
    },
    CommandDescriptor {
        kind: CommandKind::Batch,
        prefixes: &["/batch"],
        capability: Capability::Script,
        resource: Resource::Script,
    },
    CommandDescriptor {
        kind: CommandKind::Fetch,
        prefixes: &["/fetch"],
        capability: Capability::Network,
        resource: Resource::Fetch,
    },
    CommandDescriptor {
        kind: CommandKind::BypassFirewall,
        prefixes: &["/bypass-windows-firewall"],
        capability: Capability::Admin,
        resource: Resource::Admin,
    },
];
pub fn parse_legacy(raw: &str, plugins: &[String]) -> Result<ParsedCommand, AppError> {
    if raw.len() > 65536 || raw.contains('\0') {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Command is too large or contains NUL",
        ));
    }
    let normalized = raw.replace("<|§|>", " ");
    for descriptor in BUILTINS {
        if let Some(prefix) = descriptor.prefixes.iter().find(|prefix| {
            if descriptor.kind == CommandKind::BypassFirewall {
                normalized == **prefix
            } else {
                normalized.starts_with(**prefix)
            }
        }) {
            return Ok(ParsedCommand {
                kind: descriptor.kind,
                original: raw.to_string(),
                arguments: normalized[prefix.len()..].to_string(),
                normalized,
                capability: descriptor.capability,
                resource: descriptor.resource,
            });
        }
    }
    let plugin = plugins.iter().any(|p| {
        normalized
            .strip_prefix('/')
            .unwrap_or(&normalized)
            .starts_with(p)
    });
    Ok(ParsedCommand {
        kind: if plugin {
            CommandKind::Plugin
        } else {
            CommandKind::Unknown
        },
        original: raw.to_string(),
        arguments: normalized.clone(),
        normalized,
        capability: Capability::Plugin,
        resource: Resource::Plugin,
    })
}
