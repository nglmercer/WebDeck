use super::*;

#[derive(Clone)]
pub(super) struct Identity {
    pub(super) local: bool,
    pub(super) capabilities: Vec<Capability>,
}
impl Identity {
    pub(super) fn require(&self, c: Capability) -> Result<()> {
        if self.capabilities.contains(&c) {
            Ok(())
        } else {
            Err(Error::new(ErrorCode::Forbidden, "Capability denied"))
        }
    }
    pub(super) fn local(&self) -> Result<()> {
        if self.local {
            Ok(())
        } else {
            Err(Error::new(
                ErrorCode::Forbidden,
                "This operation requires a local administrator",
            ))
        }
    }
}

pub(super) fn token(h: &HeaderMap) -> Result<Option<String>> {
    match h.get("authorization") {
        None => Ok(None),
        Some(v) => {
            let s = v
                .to_str()
                .map_err(|_| Error::new(ErrorCode::Unauthorized, "Invalid credential"))?;
            let t = s
                .strip_prefix("Bearer ")
                .filter(|t| !t.is_empty())
                .ok_or_else(|| Error::new(ErrorCode::Unauthorized, "Invalid credential"))?;
            Ok(Some(t.to_string()))
        }
    }
}
pub(super) fn authority(h: &HeaderMap, local: bool, port: u16) -> Result<()> {
    let host = h
        .get("host")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| Error::new(ErrorCode::Forbidden, "Invalid host"))?;
    let parsed = reqwest::Url::parse(&format!("http://{host}")).map_err(|_| Error::invalid())?;
    let host_name = parsed.host_str().unwrap_or("").trim_matches(['[', ']']);
    let valid = host_name
        .parse::<IpAddr>()
        .is_ok_and(|a| !local || a.is_loopback())
        || (host_name == "localhost" && local);
    if parsed.port_or_known_default() != Some(port)
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(Error::new(ErrorCode::Forbidden, "Invalid server authority"));
    }
    if !valid
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.path() != "/"
    {
        return Err(Error::new(
            ErrorCode::Forbidden,
            "Host is not an approved local address",
        ));
    }
    if let Some(origin) = h.get("origin") {
        let o = origin.to_str().map_err(|_| Error::invalid())?;
        let o = reqwest::Url::parse(o).map_err(|_| Error::invalid())?;
        if !matches!(o.scheme(), "http" | "https")
            || o.host_str() != parsed.host_str()
            || o.port_or_known_default() != parsed.port_or_known_default()
        {
            return Err(Error::new(
                ErrorCode::Forbidden,
                "Cross-origin request denied",
            ));
        }
    }
    Ok(())
}
pub(super) fn network_allowed(ip: IpAddr, networks: &[String]) -> bool {
    if ip.is_loopback() || networks.is_empty() {
        return true;
    }
    networks.iter().any(|n| {
        let (a, p) = n
            .split_once('/')
            .map_or((n.as_str(), None), |(a, p)| (a, Some(p)));
        let Ok(a) = a.parse::<IpAddr>() else {
            return false;
        };
        match (ip, a) {
            (IpAddr::V4(ip), IpAddr::V4(a)) => {
                let p = match p {
                    None => 32,
                    Some(value) => match value.parse::<u32>() {
                        Ok(prefix) if prefix <= 32 => prefix,
                        _ => return false,
                    },
                };
                let m = u32::MAX.checked_shl(32 - p).unwrap_or(0);
                u32::from(ip) & m == u32::from(a) & m
            }
            (IpAddr::V6(ip), IpAddr::V6(a)) => {
                let p = match p {
                    None => 128,
                    Some(value) => match value.parse::<u32>() {
                        Ok(prefix) if prefix <= 128 => prefix,
                        _ => return false,
                    },
                };
                let m = u128::MAX.checked_shl(128 - p).unwrap_or(0);
                u128::from(ip) & m == u128::from(a) & m
            }
            _ => false,
        }
    })
}
pub(super) async fn policy_identity(
    app: &App,
    peer: IpAddr,
    credential: Option<String>,
    required: bool,
) -> Result<Option<Identity>> {
    let owner = app.clone();
    blocking(app.authorization.clone(), move || {
        if !network_allowed(
            peer,
            &owner.config.last_valid().config.settings.allowed_networks,
        ) {
            return Err(Error::new(ErrorCode::Forbidden, "Network denied"));
        }
        if !required {
            return Ok(None);
        }
        let local = peer.is_loopback();
        let capabilities = owner.sessions.authorize(credential.as_deref(), local)?;
        Ok(Some(Identity {
            local: local && credential.is_none(),
            capabilities,
        }))
    })
    .await
}

pub(super) async fn authorize(
    app: &App,
    peer: IpAddr,
    credential: Option<String>,
) -> Result<Identity> {
    policy_identity(app, peer, credential, true)
        .await?
        .ok_or_else(Error::execution)
}

pub(super) async fn guard(State(a): State<App>, mut r: Request, next: Next) -> Response {
    let result: Result<()> = async {
        let peer = r
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .ok_or_else(|| Error::new(ErrorCode::Forbidden, "Peer identity unavailable"))?
            .0;
        authority(r.headers(), peer.ip().is_loopback(), a.port)?;
        let api = r.uri().path().starts_with("/api/v2/");
        let required = api
            || (r.uri().path().starts_with("/socket.io")
                && r.headers().contains_key("authorization"));
        let credential = if required { token(r.headers())? } else { None };
        let identity = policy_identity(&a, peer.ip(), credential, required).await?;
        if api {
            r.extensions_mut()
                .insert(identity.ok_or_else(Error::execution)?);
        }
        Ok(())
    }
    .await;
    if let Err(e) = result {
        return e.into_response();
    }
    let mut response = next.run(r).await;
    let h = response.headers_mut();
    h.insert("x-content-type-options", "nosniff".parse().unwrap());
    h.insert("referrer-policy", "no-referrer".parse().unwrap());
    h.insert("cache-control", "no-store".parse().unwrap());
    response
}

#[cfg(test)]
mod tests {
    use super::network_allowed;
    #[test]
    fn malformed_network_prefixes_fail_closed() {
        for (ip, network) in [
            ("192.168.1.12", "192.168.1.0/33"),
            ("192.168.1.12", "192.168.1.12/nope"),
            ("2001:db8::1", "2001:db8::/129"),
            ("2001:db8::1", "2001:db8::1/nope"),
            ("192.168.1.12", "192.168.1.0/-1"),
            ("192.168.1.12", "192.168.1.0/4294967296"),
            ("192.168.1.12", "192.168.1.0/24/0"),
            ("192.168.1.12", "192.168.1.0/"),
            ("2001:db8::1", "2001:db8::/-1"),
            ("2001:db8::1", "2001:db8::/4294967296"),
            ("2001:db8::1", "not-an-address/0"),
        ] {
            assert!(!network_allowed(ip.parse().unwrap(), &[network.into()]));
        }
    }
    #[test]
    fn valid_network_boundaries_are_exact() {
        for (ip, network, expected) in [
            ("192.168.1.12", "192.168.1.0/24", true),
            ("192.168.2.12", "192.168.1.0/24", false),
            ("192.168.1.12", "192.168.1.12/32", true),
            ("192.168.1.13", "192.168.1.12/32", false),
            ("2001:db8::1", "2001:db8::/64", true),
            ("2001:db9::1", "2001:db8::/64", false),
            ("192.168.1.12", "0.0.0.0/0", true),
            ("2001:db8::1", "::/0", true),
        ] {
            assert_eq!(
                network_allowed(ip.parse().unwrap(), &[network.into()]),
                expected
            );
        }
    }
}
