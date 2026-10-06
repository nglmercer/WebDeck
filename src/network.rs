//! A separately managed LAN listener keeps the local settings connection alive.
use crate::{
    contracts::{ErrorCode, NetworkSettings, NetworkStatus},
    domain::{Error, Result},
    server::{router, App},
};
use std::{
    net::{IpAddr, SocketAddr, UdpSocket},
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
use tokio::{sync::Mutex, task::JoinHandle};

pub struct Network {
    path: PathBuf,
    primary_port: u16,
    external: bool,
    enabled: AtomicBool,
    state: Mutex<(NetworkSettings, Option<JoinHandle<()>>)>,
}

pub fn suggested_address() -> String {
    // Connecting a UDP socket selects a route without transmitting a packet.
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("192.0.2.1:80")?;
            s.local_addr()
        })
        .map(|a| a.ip().to_string())
        .unwrap_or_default()
}
impl Network {
    pub fn open(path: PathBuf, port: u16, external: bool) -> Result<Self> {
        let settings = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| {
                Error::new(
                    ErrorCode::PersistenceFailed,
                    "Cannot read phone access settings",
                )
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => NetworkSettings {
                enabled: external,
                address: suggested_address(),
                port: port.into(),
            },
            Err(_) => {
                return Err(Error::new(
                    ErrorCode::PersistenceFailed,
                    "Cannot read phone access settings",
                ))
            }
        };
        Ok(Self {
            path,
            primary_port: port,
            external,
            enabled: AtomicBool::new(false),
            state: Mutex::new((settings, None)),
        })
    }
    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }
    pub async fn settings(&self) -> NetworkSettings {
        self.state.lock().await.0.clone()
    }
    pub async fn status(&self) -> NetworkStatus {
        let state = self.state.lock().await;
        let mut settings = state.0.clone();
        settings.enabled = self.enabled();
        drop(state);
        let url = if settings.enabled {
            format!(
                "http://{}/",
                SocketAddr::new(
                    settings.address.parse().expect("validated address"),
                    settings.port as u16
                )
            )
        } else {
            String::new()
        };
        NetworkStatus {
            api_version: 2,
            settings,
            suggested_address: suggested_address(),
            url,
        }
    }
    pub async fn apply(&self, app: App, settings: NetworkSettings) -> Result<()> {
        crate::domain::validate(
            "NetworkSettings",
            &serde_json::to_value(&settings).map_err(|_| Error::invalid())?,
        )?;
        let address = settings.address.parse::<IpAddr>().ok();
        if settings.enabled
            && !address
                .is_some_and(|a| !a.is_loopback() && !a.is_unspecified() && !a.is_multicast())
        {
            return Err(Error::new(
                ErrorCode::InvalidInput,
                "Choose this computer's Wi-Fi or Ethernet IP address",
            ));
        }
        let mut state = self.state.lock().await;
        let unchanged = self.enabled()
            && settings.enabled
            && state.0.address == settings.address
            && state.0.port == settings.port;
        let listener = if settings.enabled
            && !unchanged
            && !(self.external && settings.port == u64::from(self.primary_port))
        {
            Some(tokio::net::TcpListener::bind(SocketAddr::new(address.expect("validated address"), settings.port as u16)).await.map_err(|_| Error::new(ErrorCode::ExecutionFailed, "Cannot enable phone access: address is not on this computer or port is busy"))?)
        } else {
            None
        };
        let bytes = serde_json::to_vec_pretty(&settings).map_err(|_| Error::invalid())?;
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || crate::storage::atomic_replace(&path, &bytes))
            .await
            .map_err(|_| Error::execution())??;
        if !unchanged {
            self.enabled.store(false, Ordering::Release);
            if let Some(task) = state.1.take() {
                task.abort();
            }
            if let Some(listener) = listener {
                let mut remote_app = app;
                remote_app.port = settings.port as u16;
                let routes = router(remote_app);
                state.1 = Some(tokio::spawn(async move {
                    if let Err(error) = axum::serve(
                        listener,
                        routes.into_make_service_with_connect_info::<SocketAddr>(),
                    )
                    .await
                    {
                        eprintln!("Phone listener: {error}");
                    }
                }));
            }
        }
        self.enabled.store(settings.enabled, Ordering::Release);
        state.0 = settings;
        Ok(())
    }
    pub async fn stop(&self) {
        self.enabled.store(false, Ordering::Release);
        if let Some(task) = self.state.lock().await.1.take() {
            task.abort();
        }
    }
}
