//! Request middleware (extracted from `server.rs`).
//!
//! Port of Flask's `@app.before_request check_local_network` and
//! `@app.after_request` hooks.

use std::net::{IpAddr, SocketAddr};

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

fn in_network(remote: IpAddr, local: IpAddr, prefix: u8) -> bool {
    match (remote, local) {
        (IpAddr::V4(remote), IpAddr::V4(local)) if prefix <= 32 => {
            let mask = if prefix == 0 {
                0
            } else {
                !0u32 << (32 - prefix)
            };
            u32::from(remote) & mask == u32::from(local) & mask
        }
        (IpAddr::V6(remote), IpAddr::V6(local)) if prefix <= 128 => {
            let mask = if prefix == 0 {
                0
            } else {
                !0u128 << (128 - prefix)
            };
            u128::from(remote) & mask == u128::from(local) & mask
        }
        _ => false,
    }
}
/// Loopback plus same-network and explicit IPv4/IPv6 entries. Invalid
/// addresses never grant access; CIDR /0 is handled without a shift panic.
fn ip_allowed(remote: IpAddr, local_ip: &str, netmask: u8, config: &Value) -> bool {
    if remote.is_loopback() {
        return true;
    }
    if local_ip
        .parse()
        .is_ok_and(|local| in_network(remote, local, netmask))
    {
        return true;
    }
    config
        .pointer("/settings/allowed_networks")
        .and_then(Value::as_array)
        .is_some_and(|networks| {
            networks.iter().filter_map(Value::as_str).any(|network| {
                if let Some((base, prefix)) = network.split_once('/') {
                    base.parse::<IpAddr>()
                        .ok()
                        .zip(prefix.parse::<u8>().ok())
                        .is_some_and(|(base, prefix)| in_network(remote, base, prefix))
                } else {
                    network.parse::<IpAddr>() == Ok(remote)
                }
            })
        })
}

/// Port of `@app.after_request` (skips `/usage`, like Python).
pub(crate) async fn after_request(req: Request<Body>, next: Next) -> Response {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let url = req.uri().path().to_string();
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

#[cfg(test)]
mod network_tests {
    use super::*;
    #[test]
    fn deny_unconfigured_ipv6_and_invalid_cidr_but_allow_explicit_networks() {
        let remote: IpAddr = "2001:db8::1".parse().unwrap();
        assert!(!ip_allowed(remote, "192.168.1.2", 16, &json!({})));
        assert!(ip_allowed(
            remote,
            "192.168.1.2",
            16,
            &json!({"settings":{"allowed_networks":["2001:db8::/32"]}})
        ));
        assert!(!in_network(
            "10.0.0.1".parse().unwrap(),
            "10.0.0.1".parse().unwrap(),
            33
        ));
        assert!(in_network(
            "10.0.0.1".parse().unwrap(),
            "192.168.0.1".parse().unwrap(),
            0
        ));
        assert!(ip_allowed(
            "::1".parse().unwrap(),
            "invalid",
            16,
            &json!({})
        ));
        assert!(!ip_allowed(
            "10.0.0.1".parse().unwrap(),
            "invalid",
            16,
            &json!({})
        ));
    }
}
