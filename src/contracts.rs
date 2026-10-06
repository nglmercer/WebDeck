// Generated. Edit contracts/v2.schema.json and run the generator.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Eq, Hash, Copy)]
pub enum Capability {
    #[serde(rename = "read")]
    Read,
    #[serde(rename = "input")]
    Input,
    #[serde(rename = "audio")]
    Audio,
    #[serde(rename = "window")]
    Window,
    #[serde(rename = "power")]
    Power,
    #[serde(rename = "script")]
    Script,
    #[serde(rename = "network")]
    Network,
    #[serde(rename = "plugin")]
    Plugin,
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "settings")]
    Settings,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Eq, Hash, Copy)]
pub enum ErrorCode {
    #[serde(rename = "invalid_input")]
    InvalidInput,
    #[serde(rename = "unsupported_schema")]
    UnsupportedSchema,
    #[serde(rename = "conflict")]
    Conflict,
    #[serde(rename = "unauthorized")]
    Unauthorized,
    #[serde(rename = "forbidden")]
    Forbidden,
    #[serde(rename = "capacity_exhausted")]
    CapacityExhausted,
    #[serde(rename = "shutting_down")]
    ShuttingDown,
    #[serde(rename = "execution_failed")]
    ExecutionFailed,
    #[serde(rename = "persistence_failed")]
    PersistenceFailed,
    #[serde(rename = "unsupported_platform")]
    UnsupportedPlatform,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum FileSource {
    #[serde(rename = "asset")]
    Asset { id: String },
    #[serde(rename = "external")]
    External { path: String },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum ScriptSource {
    #[serde(rename = "inline")]
    Inline { code: String },
    #[serde(rename = "file")]
    File { source: FileSource },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum VolumeChange {
    #[serde(rename = "set")]
    Set { percent: u64 },
    #[serde(rename = "adjust")]
    Adjust { percent: i64 },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum Command {
    #[serde(rename = "debug")]
    Debug { data: BTreeMap<String, Value> },
    #[serde(rename = "exit")]
    Exit,
    #[serde(rename = "usage")]
    Usage,
    #[serde(rename = "stop_sound")]
    StopSound,
    #[serde(rename = "play_sound")]
    PlaySound {
        source: FileSource,
        volume: u64,
        output_device: String,
        microphone: bool,
        local_only: bool,
    },
    #[serde(rename = "shutdown")]
    Shutdown,
    #[serde(rename = "reboot")]
    Reboot,
    #[serde(rename = "sleep")]
    Sleep,
    #[serde(rename = "hibernate")]
    Hibernate,
    #[serde(rename = "lock")]
    Lock,
    #[serde(rename = "screensaver_settings")]
    ScreensaverSettings,
    #[serde(rename = "screensaver")]
    Screensaver { mode: String },
    #[serde(rename = "key")]
    Key { keys: Vec<String> },
    #[serde(rename = "write")]
    Write { text: String, send: bool },
    #[serde(rename = "copy")]
    Copy { text: String, use_selection: bool },
    #[serde(rename = "paste")]
    Paste { text: String, use_selection: bool },
    #[serde(rename = "cut")]
    Cut,
    #[serde(rename = "clipboard")]
    Clipboard,
    #[serde(rename = "clear_clipboard")]
    ClearClipboard,
    #[serde(rename = "speech_recognition")]
    SpeechRecognition,
    #[serde(rename = "restart_desktop")]
    RestartDesktop,
    #[serde(rename = "close_focused")]
    CloseFocused,
    #[serde(rename = "color_picker")]
    ColorPicker,
    #[serde(rename = "kill")]
    Kill { target: String },
    #[serde(rename = "restart")]
    Restart { target: String },
    #[serde(rename = "foreground")]
    Foreground { target: String },
    #[serde(rename = "open")]
    Open { target: String },
    #[serde(rename = "volume")]
    Volume { change: VolumeChange },
    #[serde(rename = "app_volume")]
    AppVolume {
        application: String,
        change: VolumeChange,
    },
    #[serde(rename = "mute")]
    Mute,
    #[serde(rename = "play_pause")]
    PlayPause,
    #[serde(rename = "previous")]
    Previous,
    #[serde(rename = "next")]
    Next,
    #[serde(rename = "microphone")]
    Microphone { device: String },
    #[serde(rename = "speakers")]
    Speakers { device: String },
    #[serde(rename = "fetch")]
    Fetch {
        method: String,
        url: String,
        headers: BTreeMap<String, String>,
        body: String,
        timeout_seconds: u64,
    },
    #[serde(rename = "script")]
    Script {
        source: ScriptSource,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
    #[serde(rename = "shell")]
    Shell {
        source: ScriptSource,
        timeout_seconds: u64,
    },
    #[serde(rename = "firewall")]
    Firewall,
    #[serde(rename = "obs")]
    Obs { action: String, target: String },
    #[serde(rename = "spotify")]
    Spotify {
        action: String,
        target: String,
        change: VolumeChange,
    },
    #[serde(rename = "plugin")]
    Plugin {
        plugin_id: String,
        version: String,
        action_id: String,
        args: BTreeMap<String, Value>,
    },
    #[serde(rename = "button")]
    Button { button_id: String },
    #[serde(rename = "workflow")]
    Workflow { workflow: Box<WorkflowNode> },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum ButtonAction {
    #[serde(rename = "command")]
    Command { command: Command },
    #[serde(rename = "folder")]
    Folder { folder_id: String },
    #[serde(rename = "reload")]
    Reload,
    #[serde(rename = "fullscreen")]
    Fullscreen,
    #[serde(rename = "settings")]
    Settings,
    #[serde(rename = "usage")]
    Usage,
    #[serde(rename = "back")]
    Back,
    #[serde(rename = "edit")]
    Edit,
    #[serde(rename = "none")]
    None,
    #[serde(rename = "metric")]
    Metric {
        metric: String,
        target: String,
        interval_ms: u64,
    },
    #[serde(rename = "workflow")]
    Workflow { workflow: Box<WorkflowNode> },
    #[serde(rename = "script")]
    Script {
        language: String,
        source: ScriptSource,
    },
    #[serde(rename = "plugin")]
    Plugin {
        plugin_id: String,
        version: String,
        action_id: String,
        args: BTreeMap<String, Value>,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Button {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub color: String,
    pub action: ButtonAction,
    pub extensions: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Folder {
    pub id: String,
    pub label: String,
    pub buttons: Vec<Button>,
    pub extensions: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub columns: u64,
    pub rows: u64,
    pub folders: Vec<Folder>,
    pub themes: Vec<String>,
    pub backgrounds: Vec<String>,
    pub extensions: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObsSettings {
    pub host: String,
    pub port: u64,
    pub password: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpotifySettings {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub language: String,
    pub automatic_updates: bool,
    pub allowed_networks: Vec<String>,
    pub obs: ObsSettings,
    pub spotify: SpotifySettings,
    pub extensions: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub schema_version: u64,
    pub settings: Settings,
    pub layout: Layout,
    pub extensions: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandRequest {
    pub request_id: String,
    pub command: Command,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigRequest {
    pub revision: u64,
    pub config: Config,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigResponse {
    pub api_version: u64,
    pub revision: u64,
    pub config: Config,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeckBoot {
    pub api_version: u64,
    pub revision: u64,
    pub layout: Layout,
    pub language: String,
    pub can_edit: bool,
    pub capabilities: Vec<Capability>,
    pub button_capabilities: BTreeMap<String, Capability>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceRequest {
    pub name: String,
    pub capabilities: Vec<Capability>,
    pub ttl_seconds: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub capabilities: Vec<Capability>,
    pub expires_at: u64,
    pub revoked: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceApproval {
    pub api_version: u64,
    pub device: Device,
    pub token: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceList {
    pub api_version: u64,
    pub devices: Vec<Device>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandAccepted {
    pub api_version: u64,
    pub request_id: String,
    pub state: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandCompleted {
    pub api_version: u64,
    pub request_id: String,
    pub state: String,
    pub result: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandFailed {
    pub api_version: u64,
    pub request_id: String,
    pub state: String,
    pub code: ErrorCode,
    pub message: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CommandEvent {
    CommandAccepted(CommandAccepted),
    CommandCompleted(CommandCompleted),
    CommandFailed(CommandFailed),
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionRequest {
    pub revision: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FolderRequest {
    pub revision: u64,
    pub folder: Folder,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ButtonRequest {
    pub revision: u64,
    pub button: Button,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsRequest {
    pub revision: u64,
    pub settings: Settings,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeSelection {
    pub kind: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    pub schema_version: u64,
    pub id: String,
    pub version: String,
    pub entry: String,
    pub actions: Vec<PluginAction>,
    pub backend: String,
    pub digest: String,
    pub origin: String,
    pub contract: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automation: Option<AutomationMetadata>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginAction {
    pub id: String,
    pub label: String,
    pub capabilities: Vec<Capability>,
    pub arguments: BTreeMap<String, PluginArgument>,
    pub result: PluginArgument,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginArgument {
    pub r#type: String,
    pub required: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceRevocation {
    pub api_version: u64,
    pub revoked: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectionResponse {
    pub api_version: u64,
    pub source: Value,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageResponse {
    pub api_version: u64,
    pub usage: UsageSnapshot,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranslationsResponse {
    pub api_version: u64,
    pub translations: BTreeMap<String, String>,
    pub languages: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpotifyConnect {
    pub api_version: u64,
    pub url: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogResponse {
    pub api_version: u64,
    pub commands: Vec<CatalogEntry>,
    pub plugins: Vec<PluginManifest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automation: Option<AutomationMetadata>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuUsage {
    pub name: String,
    pub usage: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiskUsage {
    pub name: String,
    pub total: u64,
    pub available: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GpuUsage {
    pub name: String,
    pub usage: f64,
    pub memory_used: u64,
    pub memory_total: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageSnapshot {
    pub memory_used: u64,
    pub memory_total: u64,
    pub cpu_percent: f64,
    pub cpus: Vec<CpuUsage>,
    pub disks: Vec<DiskUsage>,
    pub gpus: Vec<GpuUsage>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioDevices {
    pub api_version: u64,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogEntry {
    pub id: String,
    pub capability: Capability,
    pub schema: BTreeMap<String, Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_schema: Option<BTreeMap<String, Value>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Eq, Hash, Copy)]
pub enum IntegrationState {
    #[serde(rename = "not_configured")]
    NotConfigured,
    #[serde(rename = "not_tested")]
    NotTested,
    #[serde(rename = "connected")]
    Connected,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "authorization_saved")]
    AuthorizationSaved,
    #[serde(rename = "authorization_required")]
    AuthorizationRequired,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegrationStatus {
    pub api_version: u64,
    pub obs: IntegrationState,
    pub spotify: IntegrationState,
    pub checked_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integrations: Option<BTreeMap<String, IntegrationHealth>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum WorkflowNode {
    #[serde(rename = "command")]
    Command {
        command: Command,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        references: Option<BTreeMap<String, String>>,
    },
    #[serde(rename = "sequence")]
    Sequence { steps: Vec<Box<WorkflowNode>> },
    #[serde(rename = "parallel")]
    Parallel { steps: Vec<Box<WorkflowNode>> },
    #[serde(rename = "conditional")]
    Conditional {
        condition: Value,
        if_true: Box<WorkflowNode>,
        if_false: Box<WorkflowNode>,
    },
    #[serde(rename = "delay")]
    Delay { milliseconds: u64 },
    #[serde(rename = "retry")]
    Retry {
        attempts: u64,
        step: Box<WorkflowNode>,
    },
    #[serde(rename = "timeout")]
    Timeout {
        milliseconds: u64,
        step: Box<WorkflowNode>,
    },
    #[serde(rename = "variable")]
    Variable { name: String, value: Value },
    #[serde(rename = "result")]
    Result { path: String },
    #[serde(rename = "script")]
    Script { source: ScriptSource },
    #[serde(rename = "plugin")]
    Plugin {
        plugin_id: String,
        version: String,
        action_id: String,
        args: BTreeMap<String, Value>,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum RuntimeEvent {
    #[serde(rename = "button.stateChanged")]
    ButtonStateChanged {
        api_version: u64,
        button_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        active: Option<bool>,
    },
    #[serde(rename = "runtime.reloaded")]
    RuntimeReloaded { api_version: u64 },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSnapshot {
    pub api_version: u64,
    pub runtime: String,
    pub healthy: bool,
    pub commands: Vec<BTreeMap<String, Value>>,
    pub plugins: Vec<PluginManifest>,
    pub loaded_plugins: Vec<String>,
    pub queue_capacity: u64,
    pub disabled_plugins: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginState {
    pub enabled: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerVersion {
    pub api_version: u64,
    pub version: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultPredicate {
    pub pointer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equals: Option<Value>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegrationDefinition {
    pub id: String,
    pub label: String,
    pub required_settings: Vec<String>,
    pub configuration: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorization_asset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub probe: Option<Command>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success: Option<ResultPredicate>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ButtonRecipe {
    pub id: String,
    pub label: String,
    pub discovery: Command,
    pub items_pointer: String,
    pub identity_pointer: String,
    pub label_pointer: String,
    pub command: Command,
    pub bindings: BTreeMap<String, String>,
    pub folder_label: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultView {
    pub items_pointer: String,
    pub columns: BTreeMap<String, String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandPresentation {
    pub selector: BTreeMap<String, Value>,
    pub label: String,
    pub arguments: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_view: Option<ResultView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_pointer: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomationMetadata {
    pub integrations: Vec<IntegrationDefinition>,
    pub button_recipes: Vec<ButtonRecipe>,
    pub presentations: Vec<CommandPresentation>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegrationHealth {
    pub state: IntegrationState,
    pub checked_at: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageAssetList {
    pub api_version: u64,
    pub images: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub live_images: Option<Vec<String>>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum ImageImport {
    #[serde(rename = "url")]
    Url {
        url: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        live: Option<bool>,
    },
    #[serde(rename = "local")]
    Local { path: String },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkSettings {
    pub enabled: bool,
    pub address: String,
    pub port: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkStatus {
    pub api_version: u64,
    pub settings: NetworkSettings,
    pub suggested_address: String,
    pub url: String,
}
impl Command {
    pub fn capability(&self) -> Capability {
        match self {
            Self::Debug { .. } => Capability::Read,
            Self::Exit => Capability::Power,
            Self::Usage => Capability::Read,
            Self::StopSound => Capability::Audio,
            Self::PlaySound { .. } => Capability::Audio,
            Self::Shutdown => Capability::Power,
            Self::Reboot => Capability::Power,
            Self::Sleep => Capability::Power,
            Self::Hibernate => Capability::Power,
            Self::Lock => Capability::Power,
            Self::ScreensaverSettings => Capability::Power,
            Self::Screensaver { .. } => Capability::Power,
            Self::Key { .. } => Capability::Input,
            Self::Write { .. } => Capability::Input,
            Self::Copy { .. } => Capability::Input,
            Self::Paste { .. } => Capability::Input,
            Self::Cut => Capability::Input,
            Self::Clipboard => Capability::Input,
            Self::ClearClipboard => Capability::Input,
            Self::SpeechRecognition => Capability::Input,
            Self::RestartDesktop => Capability::Window,
            Self::CloseFocused => Capability::Window,
            Self::ColorPicker => Capability::Window,
            Self::Kill { .. } => Capability::Window,
            Self::Restart { .. } => Capability::Window,
            Self::Foreground { .. } => Capability::Window,
            Self::Open { .. } => Capability::Window,
            Self::Volume { .. } => Capability::Audio,
            Self::AppVolume { .. } => Capability::Audio,
            Self::Mute => Capability::Audio,
            Self::PlayPause => Capability::Audio,
            Self::Previous => Capability::Audio,
            Self::Next => Capability::Audio,
            Self::Microphone { .. } => Capability::Audio,
            Self::Speakers { .. } => Capability::Audio,
            Self::Fetch { .. } => Capability::Network,
            Self::Script { .. } => Capability::Script,
            Self::Shell { .. } => Capability::Script,
            Self::Firewall => Capability::Admin,
            Self::Obs { .. } => Capability::Network,
            Self::Spotify { .. } => Capability::Network,
            Self::Plugin { .. } => Capability::Plugin,
            Self::Button { .. } => Capability::Read,
            Self::Workflow { .. } => Capability::Read,
        }
    }
}
