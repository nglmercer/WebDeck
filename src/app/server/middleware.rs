//! Request middleware (extracted from `server.rs`).
//!
//! Port of Flask's `@app.before_request check_local_network` and
//! `@app.after_request` hooks.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use serde_json::{json, Value};

use super::AppState;
use crate::app::utils::{logger::log, settings::get_config::get_config};

/// Port of `@app.before_request check_local_network`.
pub(crate) async fn check_local_network(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let config = get_config(false, false);
    let netmask: u8 = config
        .get("settings")
        .and_then(|s| s.get("netmask"))
        .and_then(|v| v.as_u64())
        .unwrap_or(16) as u8;

    // Requests without peer info (e.g. tests) are allowed through.
    if let Some(peer) = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip())
    {
        if !ip_allowed(peer, &state.local_ip, netmask, &config) {
            return (
                StatusCode::FORBIDDEN,
                Json(
                    json!({"success": false, "message": "Access denied: IP not in local network"}),
                ),
            )
                .into_response();
        }
    }

    next.run(req).await
}

fn ipv4_masked(ip: Ipv4Addr, prefix: u8) -> u32 {
    let bits = u32::from(ip);
    if prefix >= 32 {
        bits
    } else {
        bits & (!0u32 << (32 - prefix))
    }
}

/// Same-network check + `allowed_networks` (CIDR or single IP entries).
fn ip_allowed(remote: IpAddr, local_ip: &str, netmask: u8, config: &Value) -> bool {
    // Non-IPv4 remotes or unparseable local IP: allow (the app is
    // IPv4-oriented; loopback/v6 stays reachable in dev).
    let IpAddr::V4(remote_v4) = remote else {
        return true;
    };
    let Ok(local) = local_ip.parse::<Ipv4Addr>() else {
        return true;
    };

    if ipv4_masked(remote_v4, netmask) == ipv4_masked(local, netmask) {
        return true;
    }
    if let Some(networks) = config
        .get("settings")
        .and_then(|s| s.get("allowed_networks"))
        .and_then(|v| v.as_array())
    {
        for network in networks.iter().filter_map(|v| v.as_str()) {
            if let Some((base, prefix)) = network.split_once('/') {
                if let (Ok(base), Ok(prefix)) = (base.parse::<Ipv4Addr>(), prefix.parse::<u8>()) {
                    if ipv4_masked(remote_v4, prefix) == ipv4_masked(base, prefix) {
                        return true;
                    }
                }
            } else if network.parse::<IpAddr>() == Ok(remote) {
                return true;
            }
        }
    }
    false
}

/// Port of `@app.after_request` (skips `/usage`, like Python).
pub(crate) async fn after_request(req: Request<Body>, next: Next) -> Response {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let url = req.uri().to_string();
    let remote = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.to_string())
        .unwrap_or_else(|| "-".to_string());
    let response = next.run(req).await;
    if path != "/usage" {
        log().httprequest(&remote, &method, &url, response.status().as_u16());
    }
    response
}
