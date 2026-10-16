use super::*;
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Request {
    request_type: String,
    request_data: Value,
    #[serde(default)]
    allow_destructive: bool,
    #[serde(default)]
    expected_connection: Option<String>,
}
pub(super) async fn request(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Json(request): Json<Request>,
) -> Result<Json<Value>> {
    i.local()?;
    i.require(Capability::Admin)?;
    i.require(Capability::Network)?;
    if request.request_type.starts_with("Remove") && !request.allow_destructive {
        return Err(Error::new(
            ErrorCode::Forbidden,
            "Destructive OBS requests require explicit authorization",
        ));
    }
    let settings = a.config.last_valid().config.settings.obs;
    Ok(Json(
        blocking(a.io.clone(), move || {
            let connection = format!(
                "{:x}",
                <sha2::Sha256 as sha2::Digest>::digest(format!(
                    "{}:{}",
                    settings.host, settings.port
                ))
            );
            if request
                .expected_connection
                .as_ref()
                .is_some_and(|expected| expected != &connection)
            {
                return Err(Error::new(
                    ErrorCode::Conflict,
                    "Configured OBS endpoint changed during management operation",
                ));
            }
            let mut result = crate::obs::request(
                settings,
                &request.request_type,
                request.request_data,
                std::time::Instant::now() + std::time::Duration::from_secs(25),
            )?;
            if request.request_type == "GetVersion" {
                result["webdeckConnection"] = json!(connection);
            }
            Ok(result)
        })
        .await?,
    ))
}
