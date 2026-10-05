use super::*;

pub(super) async fn image_assets(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<ImageAssetList>> {
    i.local()?;
    i.require(Capability::Settings)?;
    let (images, live_images) = blocking(a.io.clone(), move || {
        let images = a.assets.image_ids()?;
        let live = images
            .iter()
            .filter(|id| crate::image_assets::is_live(&a.assets, id))
            .cloned()
            .collect();
        Ok((images, live))
    })
    .await?;
    Ok(Json(ImageAssetList {
        api_version: 2,
        images,
        live_images: Some(live_images),
    }))
}
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

pub(super) async fn import_image(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Json(r): Json<ImageImport>,
) -> Result<Json<FileSource>> {
    i.local()?;
    i.require(Capability::Settings)?;
    domain::validate(
        "ImageImport",
        &serde_json::to_value(&r).map_err(|_| Error::invalid())?,
    )?;
    Ok(Json(
        blocking(a.io.clone(), move || {
            crate::image_assets::import(&a.assets, r)
        })
        .await?,
    ))
}
pub(super) async fn refresh_image(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Path(id): Path<String>,
) -> Result<Json<FileSource>> {
    i.local()?;
    i.require(Capability::Settings)?;
    Ok(Json(
        blocking(a.io.clone(), move || {
            crate::image_assets::refresh(&a.assets, &id)
        })
        .await?,
    ))
}
