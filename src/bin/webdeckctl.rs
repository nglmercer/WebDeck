use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use webdeck::admin::{
    self, actions, buttons, configuration, doctor, folders, Emitter, Outcome, OutputMode, Result,
    WebDeckAdminClient,
};

#[derive(Parser)]
#[command(
    name = "webdeckctl",
    version,
    about = "WebDeck administration CLI",
    subcommand_required = true,
    arg_required_else_help = true
)]
struct Cli {
    #[arg(
        long,
        global = true,
        help = "Server base URL (default http://127.0.0.1:5000)"
    )]
    url: Option<String>,
    #[arg(long, global = true, help = "Bearer token for a non-local identity")]
    token: Option<String>,
    #[arg(long, global = true, help = "Emit a machine-readable JSON envelope")]
    json: bool,
    #[arg(
        short = 'v',
        long,
        global = true,
        action = clap::ArgAction::Count,
        help = "Increase log verbosity on stderr (repeatable)"
    )]
    verbose: u8,
    #[arg(
        long,
        global = true,
        help = "Server configuration directory (default .config)"
    )]
    config_dir: Option<PathBuf>,
    #[arg(
        long,
        global = true,
        help = "Accepted for compatibility; webdeckctl never prompts"
    )]
    yes: bool,
    #[arg(
        long,
        global = true,
        default_value_t = 35,
        help = "HTTP timeout in seconds"
    )]
    timeout: u64,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Show server status, revisions, plugins and integrations")]
    Status,
    #[command(about = "Run health checks against configuration, runtime and integrations")]
    Doctor,
    #[command(about = "Show the server version next to the CLI version")]
    Version,
    #[command(about = "Inspect, validate, diff, apply and export configuration")]
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    #[command(about = "List pending changes for a configuration file")]
    Plan {
        #[arg(long, help = "Configuration file, or - for stdin")]
        file: Option<String>,
    },
    #[command(about = "Apply a configuration file")]
    Apply {
        #[arg(long, help = "Configuration file, or - for stdin")]
        file: Option<String>,
        #[arg(long, help = "Expected configuration revision")]
        revision: Option<u64>,
        #[arg(long, help = "Report changes without writing")]
        dry_run: bool,
    },
    #[command(about = "Manage folders")]
    Folder {
        #[command(subcommand)]
        command: FolderCommand,
    },
    #[command(about = "Manage buttons")]
    Button {
        #[command(subcommand)]
        command: ButtonCommand,
    },
    #[command(about = "Browse and run the action catalog")]
    Action {
        #[command(subcommand)]
        command: ActionCommand,
    },
    #[command(about = "Print the v2 JSON schema")]
    Schema {
        #[arg(long, help = "Schema definition name, for example Config")]
        name: Option<String>,
    },
    #[command(about = "Show the effective capabilities for this identity")]
    Capabilities,
    #[command(about = "Manage plugin packages")]
    Plugin {
        #[command(subcommand)]
        command: PluginCommand,
    },
    #[command(about = "Manage the OBS integration")]
    Obs {
        #[command(subcommand)]
        command: ObsCommand,
    },
}

#[derive(Subcommand)]
enum ConfigCommand {
    #[command(about = "Print the current configuration (secrets redacted)")]
    Get {
        #[arg(long, help = "Include secret values in the output")]
        reveal: bool,
    },
    #[command(about = "Validate a configuration file or the server configuration")]
    Validate {
        #[arg(
            long,
            help = "Configuration file, or - for stdin (default: server config)"
        )]
        file: Option<String>,
    },
    #[command(about = "List changes between a file and the server configuration")]
    Diff {
        #[arg(long, help = "Configuration file, or - for stdin")]
        file: Option<String>,
    },
    #[command(about = "Apply a configuration file")]
    Apply {
        #[arg(long, help = "Configuration file, or - for stdin")]
        file: Option<String>,
        #[arg(long, help = "Expected configuration revision")]
        revision: Option<u64>,
        #[arg(long, help = "Report changes without writing")]
        dry_run: bool,
    },
    #[command(about = "Export the full configuration including secrets")]
    Export {
        #[arg(long, help = "Destination file (0600); omit to print to stdout")]
        file: Option<String>,
    },
}

#[derive(Args)]
struct MutationArgs {
    #[arg(long, help = "Expected configuration revision")]
    revision: Option<u64>,
    #[arg(long, help = "Report the change without writing")]
    dry_run: bool,
}

#[derive(Subcommand)]
enum FolderCommand {
    #[command(about = "List folders")]
    List,
    #[command(about = "Create a folder")]
    Create {
        #[arg(value_name = "ID", help = "Folder id (or take it from --file)")]
        id: Option<String>,
        #[arg(long, help = "Folder JSON file, or - for stdin")]
        file: Option<String>,
        #[arg(long, help = "Folder label (default: id)")]
        label: Option<String>,
        #[command(flatten)]
        mutation: MutationArgs,
    },
    #[command(about = "Update an existing folder")]
    Update {
        #[arg(value_name = "ID", help = "Folder id (or take it from --file)")]
        id: Option<String>,
        #[arg(long, help = "Folder JSON file, or - for stdin")]
        file: Option<String>,
        #[arg(long, help = "New folder label")]
        label: Option<String>,
        #[command(flatten)]
        mutation: MutationArgs,
    },
    #[command(about = "Delete a folder and its buttons")]
    Delete {
        #[arg(value_name = "ID", help = "Folder id")]
        id: String,
        #[command(flatten)]
        mutation: MutationArgs,
    },
    #[command(about = "Ensure a folder exists with the given label")]
    Ensure {
        #[arg(value_name = "ID", help = "Folder id (or take it from --file)")]
        id: Option<String>,
        #[arg(long, help = "Folder JSON file, or - for stdin")]
        file: Option<String>,
        #[arg(long, help = "Desired folder label (default: id)")]
        label: Option<String>,
        #[command(flatten)]
        mutation: MutationArgs,
    },
}

#[derive(Subcommand)]
enum ButtonCommand {
    #[command(about = "List buttons")]
    List {
        #[arg(long, help = "Only list buttons in this folder")]
        folder: Option<String>,
    },
    #[command(about = "Create a button in a folder")]
    Create {
        #[arg(long, help = "Target folder id")]
        folder: String,
        #[arg(long, help = "Button JSON file, or - for stdin")]
        file: Option<String>,
        #[command(flatten)]
        mutation: MutationArgs,
    },
    #[command(about = "Update a button with a partial JSON patch")]
    Update {
        #[arg(long, help = "Folder id")]
        folder: String,
        #[arg(long, help = "Button JSON file, or - for stdin")]
        file: Option<String>,
        #[command(flatten)]
        mutation: MutationArgs,
    },
    #[command(about = "Delete a button")]
    Delete {
        #[arg(long, help = "Folder id")]
        folder: String,
        #[arg(value_name = "ID", help = "Button id")]
        id: String,
        #[command(flatten)]
        mutation: MutationArgs,
    },
    #[command(about = "Ensure a button exists in a folder")]
    Ensure {
        #[arg(long, help = "Folder id")]
        folder: String,
        #[arg(value_name = "ID", help = "Button id (or take it from --file)")]
        id: Option<String>,
        #[arg(long, help = "Button JSON file, or - for stdin")]
        file: Option<String>,
        #[command(flatten)]
        mutation: MutationArgs,
    },
}

#[derive(Subcommand)]
enum ActionCommand {
    #[command(about = "List action types and plugin actions")]
    List,
    #[command(about = "Describe an action schema")]
    Describe {
        #[arg(value_name = "ID", help = "Action id, for example obs")]
        id: String,
    },
    #[command(about = "Run an action")]
    Run {
        #[arg(value_name = "TYPE", help = "Command type, for example obs")]
        action: Option<String>,
        #[arg(long, help = "Full command JSON file, or - for stdin")]
        file: Option<String>,
        #[arg(long, help = "Arguments as a JSON object")]
        args: Option<String>,
        #[arg(
            long = "arg",
            value_name = "KEY=VALUE",
            help = "Single argument (repeatable)"
        )]
        arg: Vec<String>,
        #[arg(long, help = "Validate and print without executing")]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum PluginCommand {
    #[command(about = "List installed and builtin plugins")]
    List,
    #[command(about = "Inspect a plugin manifest and runtime state")]
    Inspect {
        #[arg(value_name = "ID", help = "Plugin id")]
        id: String,
    },
    #[command(about = "Validate a plugin package directory or installed plugin")]
    Validate {
        #[arg(value_name = "PATH_OR_ID", help = "Package directory or plugin id")]
        target: String,
    },
    #[command(about = "Enable a plugin")]
    Enable {
        #[arg(value_name = "ID", help = "Plugin id")]
        id: String,
    },
    #[command(about = "Disable a plugin")]
    Disable {
        #[arg(value_name = "ID", help = "Plugin id")]
        id: String,
    },
    #[command(about = "Reload runtime plugins")]
    Reload,
    #[command(about = "Scaffold a new plugin package")]
    Init {
        #[arg(value_name = "ID", help = "Plugin id")]
        id: String,
        #[arg(long, help = "Target directory (default ./<id>)")]
        dir: Option<PathBuf>,
        #[arg(long, default_value = "2.0.0", help = "Plugin version")]
        version: String,
    },
    #[command(about = "Install a plugin package")]
    Install {
        #[arg(value_name = "PATH", help = "Plugin package directory")]
        path: String,
    },
    #[command(about = "Update an installed plugin from a package directory")]
    Update {
        #[arg(value_name = "PATH", help = "Plugin package directory")]
        path: String,
    },
    #[command(about = "Uninstall an installed plugin")]
    Uninstall {
        #[arg(value_name = "ID", help = "Plugin id")]
        id: String,
    },
}

#[derive(Subcommand)]
enum ObsCommand {
    #[command(about = "Show the OBS integration state")]
    Status,
    #[command(about = "Check the OBS connection")]
    Check,
    #[command(about = "Update OBS host, port or password")]
    Configure {
        #[arg(long, help = "OBS websocket host")]
        host: Option<String>,
        #[arg(long, help = "OBS websocket port")]
        port: Option<u64>,
        #[arg(long, help = "Read the OBS password from stdin")]
        password_stdin: bool,
        #[command(flatten)]
        mutation: MutationArgs,
    },
    #[command(about = "List OBS scenes")]
    Scenes,
    #[command(about = "Show the current OBS program scene")]
    CurrentScene,
    #[command(about = "List OBS inputs with mute state")]
    Inputs,
    #[command(about = "List OBS hotkeys, or trigger one with --target")]
    Hotkeys {
        #[arg(long, help = "Hotkey key id to trigger instead of listing")]
        target: Option<String>,
    },
    #[command(about = "Inspect or control the OBS stream")]
    Stream {
        #[arg(value_name = "VERB", help = "status, start, stop or toggle")]
        verb: String,
    },
    #[command(about = "Inspect or control OBS recording")]
    Recording {
        #[arg(
            value_name = "VERB",
            help = "status, start, stop, toggle, pause or resume"
        )]
        verb: String,
    },
    #[command(about = "Inspect or control the OBS virtual camera")]
    VirtualCamera {
        #[arg(value_name = "VERB", help = "status, start, stop or toggle")]
        verb: String,
    },
    #[command(about = "List OBS action ids")]
    Actions,
}

fn parse_arg(pair: &str) -> Result<(String, String)> {
    let (key, value) = pair.split_once('=').ok_or_else(|| {
        admin::AdminError::invalid_arguments(format!("--arg expects KEY=VALUE, received '{pair}'"))
    })?;
    Ok((key.to_string(), value.to_string()))
}

async fn dispatch(
    command: &Command,
    client: WebDeckAdminClient,
    connection: &admin::Connection,
    cli: &Cli,
) -> Result<Outcome> {
    let _ = cli.yes;
    match command {
        Command::Status => doctor::status(&client).await,
        Command::Doctor => doctor::doctor(&client).await,
        Command::Version => {
            let version = client.version().await?;
            let data = serde_json::json!({
                "api_version": version.api_version,
                "version": version.version,
                "cli_version": env!("CARGO_PKG_VERSION")
            });
            Ok(Outcome::ok(
                data,
                format!(
                    "server {}, api v{}, cli {}",
                    version.version,
                    version.api_version,
                    env!("CARGO_PKG_VERSION")
                ),
            ))
        }
        Command::Config { command } => match command {
            ConfigCommand::Get { reveal } => configuration::get(&client, *reveal).await,
            ConfigCommand::Validate { file } => {
                configuration::validate(&client, file.as_deref()).await
            }
            ConfigCommand::Diff { file } => configuration::diff(&client, file.as_deref()).await,
            ConfigCommand::Apply {
                file,
                revision,
                dry_run,
            } => configuration::apply(&client, file.as_deref(), *revision, *dry_run).await,
            ConfigCommand::Export { file } => configuration::export(&client, file.as_deref()).await,
        },
        Command::Plan { file } => configuration::diff(&client, file.as_deref()).await,
        Command::Apply {
            file,
            revision,
            dry_run,
        } => configuration::apply(&client, file.as_deref(), *revision, *dry_run).await,
        Command::Folder { command } => match command {
            FolderCommand::List => folders::list(&client).await,
            FolderCommand::Create {
                id,
                file,
                label,
                mutation,
            } => {
                folders::create(
                    &client,
                    file.as_deref(),
                    id.as_deref(),
                    label.as_deref(),
                    mutation.revision,
                    mutation.dry_run,
                )
                .await
            }
            FolderCommand::Update {
                id,
                file,
                label,
                mutation,
            } => {
                folders::update(
                    &client,
                    file.as_deref(),
                    id.as_deref(),
                    label.as_deref(),
                    mutation.revision,
                    mutation.dry_run,
                )
                .await
            }
            FolderCommand::Delete { id, mutation } => {
                folders::delete(&client, id, mutation.revision, mutation.dry_run).await
            }
            FolderCommand::Ensure {
                id,
                file,
                label,
                mutation,
            } => {
                folders::ensure(
                    &client,
                    file.as_deref(),
                    id.as_deref(),
                    label.as_deref(),
                    mutation.revision,
                    mutation.dry_run,
                )
                .await
            }
        },
        Command::Button { command } => match command {
            ButtonCommand::List { folder } => buttons::list(&client, folder.as_deref()).await,
            ButtonCommand::Create {
                folder,
                file,
                mutation,
            } => {
                buttons::create(
                    &client,
                    folder,
                    file.as_deref(),
                    mutation.revision,
                    mutation.dry_run,
                )
                .await
            }
            ButtonCommand::Update {
                folder,
                file,
                mutation,
            } => {
                buttons::update(
                    &client,
                    folder,
                    file.as_deref(),
                    mutation.revision,
                    mutation.dry_run,
                )
                .await
            }
            ButtonCommand::Delete {
                folder,
                id,
                mutation,
            } => buttons::delete(&client, folder, id, mutation.revision, mutation.dry_run).await,
            ButtonCommand::Ensure {
                folder,
                id,
                file,
                mutation,
            } => {
                buttons::ensure(
                    &client,
                    folder,
                    file.as_deref(),
                    id.as_deref(),
                    mutation.revision,
                    mutation.dry_run,
                )
                .await
            }
        },
        Command::Action { command } => match command {
            ActionCommand::List => actions::list(&client).await,
            ActionCommand::Describe { id } => actions::describe(&client, id).await,
            ActionCommand::Run {
                action,
                file,
                args,
                arg,
                dry_run,
            } => {
                let pairs: Result<Vec<(String, String)>> =
                    arg.iter().map(|p| parse_arg(p)).collect();
                let pairs = pairs?;
                actions::run(
                    &client,
                    action.as_deref(),
                    file.as_deref(),
                    args.as_deref(),
                    &pairs,
                    *dry_run,
                )
                .await
            }
        },
        Command::Schema { name } => doctor::schema(name.as_deref()),
        Command::Capabilities => doctor::capabilities(&client).await,
        Command::Plugin { command } => {
            let service = client.plugins(connection.config_dir.clone());
            match command {
                PluginCommand::List => service.list().await,
                PluginCommand::Inspect { id } => service.inspect(id).await,
                PluginCommand::Validate { target } => service.validate_package(target),
                PluginCommand::Enable { id } => service.set_enabled(id, true).await,
                PluginCommand::Disable { id } => service.set_enabled(id, false).await,
                PluginCommand::Reload => service.reload().await,
                PluginCommand::Init { id, dir, version } => service.init(id, dir.clone(), version),
                PluginCommand::Install { path } => service.install(path).await,
                PluginCommand::Update { path } => service.update(path).await,
                PluginCommand::Uninstall { id } => service.uninstall(id).await,
            }
        }
        Command::Obs { command } => {
            let obs = client.obs();
            match command {
                ObsCommand::Status => obs.status().await,
                ObsCommand::Check => obs.check().await,
                ObsCommand::Configure {
                    host,
                    port,
                    password_stdin,
                    mutation,
                } => {
                    obs.configure(
                        host.as_deref(),
                        *port,
                        *password_stdin,
                        mutation.revision,
                        mutation.dry_run,
                    )
                    .await
                }
                ObsCommand::Scenes => obs.scenes().await,
                ObsCommand::CurrentScene => obs.current_scene().await,
                ObsCommand::Inputs => obs.inputs().await,
                ObsCommand::Hotkeys { target } => obs.hotkeys(target.as_deref()).await,
                ObsCommand::Stream { verb } => obs.stream(verb).await,
                ObsCommand::Recording { verb } => obs.recording(verb).await,
                ObsCommand::VirtualCamera { verb } => obs.virtual_camera(verb).await,
                ObsCommand::Actions => obs.actions().await,
            }
        }
    }
}

async fn run(cli: Cli, emitter: &Emitter) -> i32 {
    let connection = match admin::resolve_connection(
        cli.url.as_deref(),
        cli.token.as_deref(),
        cli.config_dir.clone(),
    ) {
        Ok(connection) => connection,
        Err(error) => return emitter.emit_error(&error),
    };
    let client = admin::connect(&connection, cli.timeout, cli.verbose);
    match dispatch(&cli.command, client, &connection, &cli).await {
        Ok(outcome) => emitter.emit(&outcome),
        Err(error) => emitter.emit_error(&error),
    }
}

fn main() {
    let cli = Cli::parse();
    let emitter = Emitter::new(if cli.json {
        OutputMode::Json
    } else {
        OutputMode::Human
    });
    let code = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
        .block_on(run(cli, &emitter));
    std::process::exit(code);
}
