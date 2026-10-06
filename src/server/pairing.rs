use super::*;
pub(super) async fn request(
    State(a): State<App>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(value): Json<Value>,
) -> Result<Json<PairingChallenge>> {
    domain::validate("PairingRequest", &value)?;
    let r: PairingRequest = serde_json::from_value(value).map_err(|_| Error::invalid())?;
    Ok(Json(
        blocking(a.authorization.clone(), move || {
            a.sessions.request_pairing(r.name, peer.ip())
        })
        .await?,
    ))
}
pub(super) async fn claim(
    State(a): State<App>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(value): Json<Value>,
) -> Result<Json<PairingResult>> {
    domain::validate("PairingClaim", &value)?;
    let r = serde_json::from_value(value).map_err(|_| Error::invalid())?;
    Ok(Json(
        blocking(a.authorization.clone(), move || {
            a.sessions.claim_pairing(r, peer.ip())
        })
        .await?,
    ))
}
pub(super) async fn pending(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<PairingList>> {
    i.local()?;
    i.require(Capability::Settings)?;
    Ok(Json(
        blocking(a.authorization.clone(), move || {
            Ok(a.sessions.pending_pairings())
        })
        .await?,
    ))
}
pub(super) async fn accept(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Path(id): Path<String>,
    Json(value): Json<Value>,
) -> Result<Json<PairingList>> {
    i.local()?;
    i.require(Capability::Settings)?;
    domain::validate("DeviceRequest", &value)?;
    let r = serde_json::from_value(value).map_err(|_| Error::invalid())?;
    Ok(Json(
        blocking(a.io.clone(), move || {
            a.sessions.accept_pairing(&id, r)?;
            Ok(a.sessions.pending_pairings())
        })
        .await?,
    ))
}
pub(super) async fn reject(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Path(id): Path<String>,
) -> Result<Json<PairingList>> {
    i.local()?;
    i.require(Capability::Settings)?;
    Ok(Json(
        blocking(a.authorization.clone(), move || {
            a.sessions.reject_pairing(&id)?;
            Ok(a.sessions.pending_pairings())
        })
        .await?,
    ))
}
