use clap::Parser;
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use webdeck::{
    domain::Result,
    executor::{Adapter, Executor, Fake},
    native::Native,
    server::{router, App},
    sessions::Sessions,
    storage::{Assets, ConfigStore},
};
#[derive(Parser)]
#[command(name = "WebDeck", version)]
struct Args {
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: std::net::IpAddr,
    #[arg(short = 'p', long, default_value_t = 5000)]
    port: u16,
    #[arg(long)]
    no_tray: bool,
    #[arg(long)]
    config_dir: Option<PathBuf>,
    #[arg(long)]
    force_start: bool,
    #[arg(long)]
    no_admin: bool,
    #[arg(long)]
    no_auto_update: bool,
}
#[tokio::main]
async fn main() {
    if let Err(e) = start().await {
        eprintln!("WebDeck: {e}");
        std::process::exit(1);
    }
}
async fn start() -> Result<()> {
    let args = Args::parse();
    let dir = args
        .config_dir
        .or_else(|| std::env::var_os("WEBDECK_CONFIG_DIR").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(".config"));
    let config = Arc::new(ConfigStore::open(dir.join("config.json"))?);
    let sessions = Arc::new(Sessions::open(dir.join("devices.v2.json"))?);
    let assets = Assets { root: dir };
    let shutdown = Arc::new(tokio::sync::Notify::new());
    let native = Arc::new(Native::new(
        config.clone(),
        assets.clone(),
        shutdown.clone(),
    )?);
    let plugins = Arc::new(native.plugins.iter().map(|p| p.manifest.clone()).collect());
    let adapter: Arc<dyn Adapter> =
        if cfg!(debug_assertions) && std::env::var("WEBDECK_FAKE_EFFECTS").as_deref() == Ok("1") {
            Arc::new(Fake)
        } else {
            Arc::new(native)
        };
    let executor = Arc::new(Executor::new(adapter, 16));
    let app = App {
        io: Arc::new(tokio::sync::Semaphore::new(4)),
        queries: Arc::new(tokio::sync::Semaphore::new(2)),
        authorization: Arc::new(tokio::sync::Semaphore::new(4)),
        port: args.port,
        plugins,
        config,
        sessions,
        executor: executor.clone(),
        assets,
    };
    let addr = SocketAddr::new(args.host, args.port);
    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|_| {
        webdeck::domain::Error::new(
            webdeck::contracts::ErrorCode::ExecutionFailed,
            "Cannot bind server; another instance may be running",
        )
    })?;
    let local = if args.host.is_unspecified() {
        SocketAddr::new("127.0.0.1".parse().expect("loopback"), args.port)
    } else {
        addr
    };
    let url = format!("http://{local}/");
    println!("WebDeck {} listening at {url}", env!("CARGO_PKG_VERSION"));
    if !args.no_tray {
        let notify = shutdown.clone();
        std::thread::spawn(move || {
            if let Err(e) = webdeck::desktop::run(url, notify) {
                eprintln!("Tray: {e}");
            }
        });
    }
    let signal = shutdown.clone();
    axum::serve(
        listener,
        router(app).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        tokio::select! {_=tokio::signal::ctrl_c()=>{},_=signal.notified()=>{}}
    })
    .await
    .map_err(|_| webdeck::domain::Error::execution())?;
    executor.drain().await;
    Ok(())
}
