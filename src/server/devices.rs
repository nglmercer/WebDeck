use super::*;

pub(super) async fn devices(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<DeviceList>> {
    i.local()?;
    i.require(Capability::Settings)?;
    Ok(Json(DeviceList {
        api_version: 2,
        devices: blocking(a.io.clone(), move || Ok(a.sessions.list())).await?,
    }))
}
pub(super) async fn approve(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Json(r): Json<DeviceRequest>,
) -> Result<Json<DeviceApproval>> {
    i.local()?;
    i.require(Capability::Settings)?;
    Ok(Json(
        blocking(a.io.clone(), move || a.sessions.approve(r)).await?,
    ))
}
pub(super) async fn revoke(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    i.local()?;
    i.require(Capability::Settings)?;
    blocking(a.io.clone(), move || a.sessions.revoke(&id)).await?;
    Ok(Json(json!({"api_version":2,"revoked":true})))
}
