use std::net::{IpAddr, SocketAddr};

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{HeaderMap, Request},
    middleware::Next,
    response::Response,
};

use super::AppState;
use crate::application::sessions;
use crate::domain::{
    command::Capability,
    error::{AppError, ErrorCode},
};

#[derive(Clone, Debug)]
pub struct Identity {
    pub capabilities: Option<Vec<Capability>>,
}

pub(crate) fn is_local(peer: Option<IpAddr>) -> bool {
    peer.is_some_and(|ip| ip.is_loopback())
}

/// Do not trust Forwarded/X-Forwarded-For for local administrative approval.
pub(crate) fn origin_allowed(headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get("origin") else {
        return true;
    };
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    let Ok(uri) = origin.parse::<axum::http::Uri>() else {
        return false;
    };
    if !matches!(uri.scheme_str(), Some("http" | "https")) {
        return false;
    }
    let Some(host) = headers.get("host").and_then(|h| h.to_str().ok()) else {
        return false;
    };
    uri.authority()
        .is_some_and(|authority| authority.as_str().eq_ignore_ascii_case(host))
        && uri.query().is_none()
        && (uri.path().is_empty() || uri.path() == "/")
}

pub(crate) fn strict_mode() -> bool {
    crate::app::utils::settings::get_config::get_config(false, false)
        .pointer("/settings/v2_security")
        .and_then(|v| v.as_str())
        == Some("paired")
}

pub(crate) fn authorize(
    token: Option<&str>,
    local: bool,
    legacy: bool,
) -> Result<Identity, AppError> {
    if let Some(token) = token {
        return Ok(Identity {
            capabilities: Some(sessions::shared()?.authorize(token)?),
        });
    }
    if local || (legacy && !strict_mode()) {
        return Ok(Identity { capabilities: None });
    }
    Err(AppError::new(
        ErrorCode::Unauthorized,
        "A paired device identity is required",
    ))
}

pub(crate) fn require(identity: &Identity, capability: Capability) -> Result<(), AppError> {
    if identity
        .capabilities
        .as_ref()
        .is_some_and(|caps| !caps.contains(&capability))
    {
        Err(AppError::new(ErrorCode::Forbidden, "Capability denied"))
    } else {
        Ok(())
    }
}

pub(crate) async fn guard(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip());
    let local = is_local(peer);
    if !origin_allowed(request.headers())
        || (local && !local_browser_origin(request.headers(), &state.local_ip))
    {
        return super::v2::error_response(
            AppError::new(ErrorCode::Forbidden, "Cross-origin requests are denied"),
            None,
        );
    }
    let path = request.uri().path();
    // Namespace middleware authenticates Socket.IO auth payloads, and every
    // subsequent command rechecks expiry/revocation. The HTTP layer still
    // applies network, Origin and size policy to Engine.IO handshakes.
    if path.starts_with("/socket.io")
        || path == "/"
        || path.starts_with("/assets/")
        || path.starts_with("/static/")
    {
        return next.run(request).await;
    }
    if path.starts_with("/api/v2/devices") && !local {
        return super::v2::error_response(
            AppError::new(
                ErrorCode::Forbidden,
                "Device approval requires the local administrator",
            ),
            None,
        );
    }
    let token = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    let identity = match authorize(token, local, !path.starts_with("/api/v2/")) {
        Ok(identity) => identity,
        Err(error) => return super::v2::error_response(error, None),
    };
    // Legacy saves and file pickers also use capability policy when paired.
    let capability = if matches!(
        path,
        "/save_config"
            | "/COMPLETE_save_config"
            | "/save_single_button"
            | "/save_buttons_only"
            | "/create_folder"
            | "/get_config"
            | "/api/boot"
            | "/upload_file"
            | "/upload_filepath"
            | "/upload_folderpath"
            | "/api/v2/config"
    ) {
        Capability::Settings
    } else {
        Capability::Read
    };
    if !(path == "/send-data"
        || (path == "/api/v2/commands" && request.method() == axum::http::Method::POST))
    {
        if let Err(error) = require(&identity, capability) {
            return super::v2::error_response(error, None);
        }
    }
    // Settings grants may edit layout/integration preferences, but cannot
    // weaken network policy, grant privileges, or redirect the updater.
    if request.method() == axum::http::Method::POST
        && matches!(
            path,
            "/save_config" | "/COMPLETE_save_config" | "/api/v2/config"
        )
        && identity.capabilities.is_some()
    {
        let (parts, body) = request.into_parts();
        let bytes = match axum::body::to_bytes(body, 16 * 1024 * 1024).await {
            Ok(bytes) => bytes,
            Err(_) => {
                return super::v2::error_response(
                    AppError::new(ErrorCode::InvalidInput, "Invalid request body"),
                    None,
                )
            }
        };
        if let Ok(mut submitted) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            if parts.uri.path() == "/api/v2/config" {
                submitted = submitted["config"].take();
            }
            let old = crate::app::utils::settings::get_config::get_config(false, false);
            let replacement = parts.uri.path() != "/save_config";
            let protected = [
                "v2_security",
                "allowed_networks",
                "netmask",
                "automatic_firewall_bypass",
                "app_admin",
                "windows_startup",
                "windows_start_menu_shortcut",
                "update_repo",
                "update_channel",
                "auto_updates",
            ];
            if protected.iter().any(|key| {
                let next = submitted.pointer(&format!("/settings/{key}"));
                (replacement || next.is_some()) && next != old.pointer(&format!("/settings/{key}"))
            }) {
                return super::v2::error_response(
                    AppError::new(
                        ErrorCode::Forbidden,
                        "Administrative configuration requires local approval",
                    ),
                    None,
                );
            }
        }
        request = Request::from_parts(parts, Body::from(bytes));
    }
    request.extensions_mut().insert(identity);
    next.run(request).await
}

// A matching Origin/Host alone permits DNS rebinding into loopback admin
// trust. Local browser requests must name localhost or a known server IP.
fn local_browser_origin(headers: &HeaderMap, local_ip: &str) -> bool {
    let Some(origin) = headers.get("origin") else {
        return true;
    };
    let Some(uri) = origin
        .to_str()
        .ok()
        .and_then(|s| s.parse::<axum::http::Uri>().ok())
    else {
        return false;
    };
    let Some(host) = uri.host() else {
        return false;
    };
    host.eq_ignore_ascii_case("localhost")
        || host
            .trim_matches(['[', ']'])
            .parse::<IpAddr>()
            .is_ok_and(|ip| {
                ip.is_loopback()
                    || ip.to_string() == local_ip
                    || crate::app::utils::args::get_args().host.as_deref() == Some(host)
            })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_browser_origin_rejects_rebinding_host_even_with_matching_origin() {
        for (origin, allowed) in [
            ("http://localhost:5000", true),
            ("http://127.0.0.1:5000", true),
            ("http://[::1]:5000", true),
            ("http://192.168.1.2:5000", true),
            ("http://webdeck.test:5000", false),
        ] {
            let mut headers = HeaderMap::new();
            headers.insert("origin", origin.parse().unwrap());
            assert_eq!(local_browser_origin(&headers, "192.168.1.2"), allowed);
        }
    }
}
