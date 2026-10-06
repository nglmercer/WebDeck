use clap::Parser;
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use webdeck::{
    capabilities::Platform,
    domain::Result,
    executor::{Adapter, Executor},
    runtime::{capabilities::NativeMetrics, VmAdapter, VmRuntime},
    server::{router, App},
    sessions::Sessions,
    storage::{Assets, ConfigStore},
};
#[derive(Parser)]
#[command(name = "WebDeck", version)]
struct Args {
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: std::net::IpAddr,
    /// Listen on all IPv4 interfaces so paired devices can connect over LAN.
    #[arg(long, conflicts_with = "host")]
    lan: bool,
    /// Phone-facing URL for the tray QR code (for example http://192.168.1.10:5000/).
    #[arg(long)]
    qr_url: Option<reqwest::Url>,
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
    let network = Arc::new(webdeck::network::Network::open(
        dir.join("network.v2.json"),
        args.port,
        args.lan || !args.host.is_loopback(),
    )?);
    let assets = Assets { root: dir };
    let shutdown = Arc::new(tokio::sync::Notify::new());
    let native = Arc::new(Platform::new(
        config.clone(),
        assets.clone(),
        shutdown.clone(),
    )?);
    let (loaded, rejected) = webdeck::runtime::plugins::discover_plugins(&assets)?;
    if rejected > 0 {
        eprintln!("WebDeck: {rejected} plugin packages were quarantined; migrate or repair their v2 manifests before reloading");
    }
    let plugins = Arc::new(
        loaded
            .iter()
            .map(|plugin| plugin.manifest.clone())
            .collect(),
    );
    let host: Arc<dyn webdeck::runtime::capabilities::CapabilityHost> =
        if cfg!(debug_assertions) && std::env::var("WEBDECK_FAKE_EFFECTS").as_deref() == Ok("1") {
            Arc::new(webdeck::capabilities::FakePlatform::new(native.clone()))
        } else {
            native
        };
    let runtime = Arc::new(VmRuntime::with_plugins(
        Arc::new(NativeMetrics),
        host,
        loaded,
    )?);
    let adapter: Arc<dyn Adapter> = Arc::new(VmAdapter::new(runtime));
    let executor = Arc::new(Executor::new(adapter, 16));
    let app = App {
        network: Some(network.clone()),
        integration_health: Default::default(),
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
    let bind_host = if args.lan {
        std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED)
    } else {
        args.host
    };
    if let Some(url) = &args.qr_url {
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(webdeck::domain::Error::new(
                webdeck::contracts::ErrorCode::ExecutionFailed,
                "QR URL must be an HTTP(S) address without credentials, query or fragment",
            ));
        }
    }
    let addr = SocketAddr::new(bind_host, args.port);
    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|_| {
        webdeck::domain::Error::new(
            webdeck::contracts::ErrorCode::ExecutionFailed,
            "Cannot bind server; another instance may be running",
        )
    })?;
    let local = if bind_host.is_unspecified() {
        SocketAddr::new("127.0.0.1".parse().expect("loopback"), args.port)
    } else {
        addr
    };
    let url = format!("http://{local}/");
    println!("WebDeck {} listening at {url}", env!("CARGO_PKG_VERSION"));
    let qr_url = args
        .qr_url
        .map(|url| url.to_string())
        .unwrap_or_else(|| url.clone());
    if bind_host.is_unspecified() {
        println!(
            "LAN access enabled on port {}. Pair your phone in local Settings → Devices.",
            args.port
        );
        if qr_url == url {
            println!(
                "Use --qr-url http://YOUR_LAN_IP:{}/ for a phone-accessible QR code.",
                args.port
            );
        }
    }
    let saved_network = network.settings().await;
    if let Err(error) = network.apply(app.clone(), saved_network).await {
        eprintln!("Phone access: {error}. Open local Settings → Connection to fix it.");
    }
    if !args.no_tray {
        let notify = shutdown.clone();
        let phone_network = network.clone();
        let handle = tokio::runtime::Handle::current();
        std::thread::spawn(move || {
            if let Err(e) = webdeck::desktop::run(url, qr_url, phone_network, handle, notify) {
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
    network.stop().await;
    executor.drain().await;
    Ok(())
}
