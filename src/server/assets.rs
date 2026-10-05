use super::*;

pub(super) async fn upload(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    mut m: Multipart,
) -> Result<Json<FileSource>> {
    i.require(Capability::Settings)?;
    let f = m
        .next_field()
        .await
        .map_err(|_| Error::invalid())?
        .ok_or_else(Error::invalid)?;
    let extension = f
        .file_name()
        .and_then(|n| n.rsplit('.').next())
        .unwrap_or("")
        .to_lowercase();
    let bytes = f.bytes().await.map_err(|_| Error::invalid())?;
    Ok(Json(
        blocking(a.io.clone(), move || a.assets.upload(&extension, &bytes)).await?,
    ))
}
pub(super) async fn asset(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Path(id): Path<String>,
) -> Result<Response> {
    i.require(Capability::Read)?;
    let assets = a.assets.clone();
    let asset_id = id.clone();
    let b = blocking(a.io.clone(), move || assets.read(&asset_id)).await?;
    let mime = match id.rsplit('.').next().unwrap_or("") {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "css" => "text/css; charset=utf-8",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    };
    Ok((
        [
            ("content-type", mime),
            ("content-security-policy", "sandbox; default-src 'none'"),
        ],
        b,
    )
        .into_response())
}
pub(super) async fn selection(
    axum::Extension(i): axum::Extension<Identity>,
    Json(r): Json<NativeSelection>,
) -> Result<Json<Value>> {
    i.local()?;
    i.require(Capability::Settings)?;
    domain::validate(
        "NativeSelection",
        &serde_json::to_value(&r).map_err(|_| Error::invalid())?,
    )?;
    let p = if r.kind == "folder" {
        rfd::AsyncFileDialog::new().pick_folder().await
    } else {
        rfd::AsyncFileDialog::new().pick_file().await
    };
    Ok(Json(
        json!({"api_version":2,"source":p.map(|p|FileSource::External{path:p.path().to_string_lossy().into_owned()})}),
    ))
}
