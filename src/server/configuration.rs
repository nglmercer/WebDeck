use super::*;

pub(super) async fn config(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<ConfigResponse>> {
    i.local()?;
    i.require(Capability::Settings)?;
    Ok(Json(
        blocking(a.io.clone(), move || a.config.snapshot()).await?,
    ))
}
pub(super) async fn replace(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Json(r): Json<ConfigRequest>,
) -> Result<Json<ConfigResponse>> {
    i.local()?;
    i.require(Capability::Settings)?;
    Ok(Json(
        blocking(a.io.clone(), move || {
            let previous = a.config.last_valid().config.settings;
            let response = a.config.mutate(r.revision, |c| {
                *c = r.config;
                Ok(())
            })?;
            if previous != response.config.settings {
                a.integration_health
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .invalidate_changed(&response.config.settings);
            }
            Ok(response)
        })
        .await?,
    ))
}
pub(super) async fn settings(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Json(r): Json<SettingsRequest>,
) -> Result<Json<ConfigResponse>> {
    i.local()?;
    i.require(Capability::Settings)?;
    Ok(Json(
        blocking(a.io.clone(), move || {
            let previous = a.config.last_valid().config.settings;
            let response = a.config.mutate(r.revision, |c| {
                c.settings = r.settings;
                Ok(())
            })?;
            if previous != response.config.settings {
                a.integration_health
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .invalidate_changed(&response.config.settings);
            }
            Ok(response)
        })
        .await?,
    ))
}
pub(super) async fn folder(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Json(r): Json<FolderRequest>,
) -> Result<Json<ConfigResponse>> {
    i.local()?;
    i.require(Capability::Settings)?;
    Ok(Json(
        blocking(a.io.clone(), move || {
            a.config.mutate(r.revision, |c| {
                c.layout.folders.push(r.folder);
                Ok(())
            })
        })
        .await?,
    ))
}
pub(super) async fn remove_folder(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Path(id): Path<String>,
    Json(r): Json<RevisionRequest>,
) -> Result<Json<ConfigResponse>> {
    i.local()?;
    i.require(Capability::Settings)?;
    Ok(Json(
        blocking(a.io.clone(), move || {
            a.config.mutate(r.revision, |c| {
                if !c.layout.folders.iter().any(|f| f.id == id) {
                    return Err(Error::invalid());
                }
                c.layout.folders.retain(|f| f.id != id);
                Ok(())
            })
        })
        .await?,
    ))
}
pub(super) async fn button(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Path(id): Path<String>,
    Json(r): Json<ButtonRequest>,
) -> Result<Json<ConfigResponse>> {
    i.local()?;
    i.require(Capability::Settings)?;
    Ok(Json(
        blocking(a.io.clone(), move || {
            a.config.mutate(r.revision, |c| {
                c.layout
                    .folders
                    .iter_mut()
                    .find(|f| f.id == id)
                    .ok_or_else(Error::invalid)?
                    .buttons
                    .push(r.button);
                Ok(())
            })
        })
        .await?,
    ))
}
pub(super) async fn update_button(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Path((folder, id)): Path<(String, String)>,
    Json(r): Json<ButtonRequest>,
) -> Result<Json<ConfigResponse>> {
    i.local()?;
    i.require(Capability::Settings)?;
    if r.button.id != id {
        return Err(Error::invalid());
    }
    Ok(Json(
        blocking(a.io.clone(), move || {
            a.config.mutate(r.revision, |c| {
                let f = c
                    .layout
                    .folders
                    .iter_mut()
                    .find(|f| f.id == folder)
                    .ok_or_else(Error::invalid)?;
                *f.buttons
                    .iter_mut()
                    .find(|b| b.id == id)
                    .ok_or_else(Error::invalid)? = r.button;
                Ok(())
            })
        })
        .await?,
    ))
}
pub(super) async fn remove_button(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Path((folder, id)): Path<(String, String)>,
    Json(r): Json<RevisionRequest>,
) -> Result<Json<ConfigResponse>> {
    i.local()?;
    i.require(Capability::Settings)?;
    Ok(Json(
        blocking(a.io.clone(), move || {
            a.config.mutate(r.revision, |c| {
                let f = c
                    .layout
                    .folders
                    .iter_mut()
                    .find(|f| f.id == folder)
                    .ok_or_else(Error::invalid)?;
                if !f.buttons.iter().any(|b| b.id == id) {
                    return Err(Error::invalid());
                }
                f.buttons.retain(|b| b.id != id);
                Ok(())
            })
        })
        .await?,
    ))
}
