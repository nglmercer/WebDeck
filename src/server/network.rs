use super::*;
fn manager(a: &App, i: &Identity) -> Result<Arc<crate::network::Network>> {
    i.local()?;
    i.require(Capability::Settings)?;
    a.network.clone().ok_or_else(|| {
        Error::new(
            ErrorCode::ExecutionFailed,
            "Phone settings are unavailable in this server",
        )
    })
}
pub(super) async fn status(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<NetworkStatus>> {
    Ok(Json(manager(&a, &i)?.status().await))
}
pub(super) async fn apply(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Json(v): Json<Value>,
) -> Result<Json<NetworkStatus>> {
    let network = manager(&a, &i)?;
    domain::validate("NetworkSettings", &v)?;
    let settings = serde_json::from_value(v).map_err(|_| Error::invalid())?;
    network.apply(a, settings).await?;
    Ok(Json(network.status().await))
}
pub(super) async fn qr(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Response> {
    let status = manager(&a, &i)?.status().await;
    if !status.settings.enabled {
        return Err(Error::new(
            ErrorCode::InvalidInput,
            "Enable phone access to show its QR code",
        ));
    }
    let png = crate::qr::generate_qr_code_png(&status.url).ok_or_else(Error::execution)?;
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "image/png"),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        png,
    )
        .into_response())
}
