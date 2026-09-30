//! Upload + user-file routes (extracted from `server.rs`).
//!
//! `POST /upload_folderpath`, `POST /upload_filepath`, `POST /upload_file`,
//! `GET /.config/<directory>/<filename>`.

use std::collections::HashMap;

use axum::{
    body::Body,
    extract::{Multipart, Path, Query},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;

use super::assets::save_rotated_copy;
use super::internal_error;
use crate::adapters::{config::atomic_replace, files::confined_path};
use crate::app::utils::settings::get_config::config_dir;
use crate::app::utils::{languages::text, logger::log};

/// Port of `upload_folderpath` (`POST /upload_folderpath`).
/// `easygui.diropenbox` → `rfd` folder picker (`""` on cancel, like Python).
pub(crate) async fn upload_folderpath() -> String {
    // Native dialogs are blocking; run off the async runtime.
    tokio::task::spawn_blocking(|| {
        rfd::FileDialog::new()
            .pick_folder()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default()
    })
    .await
    .unwrap_or_default()
}

/// Port of `upload_filepath` (`POST /upload_filepath`).
/// `easygui.fileopenbox` → `rfd` file picker (`""` on cancel, like Python).
pub(crate) async fn upload_filepath(Query(params): Query<HashMap<String, String>>) -> String {
    let filetypes = params.get("filetypes").cloned().unwrap_or_default();
    tokio::task::spawn_blocking(move || {
        let mut dialog = rfd::FileDialog::new();
        if !filetypes.is_empty() {
            // Python: `filetypes.split('_')` → `*ext` easygui patterns.
            let extensions: Vec<String> = filetypes
                .split('_')
                .map(|item| {
                    item.trim()
                        .trim_start_matches('.')
                        .trim_start_matches('*')
                        .to_string()
                })
                .filter(|item| !item.is_empty())
                .collect();
            let borrowed: Vec<&str> = extensions.iter().map(|s| s.as_str()).collect();
            if !borrowed.is_empty() {
                dialog = dialog.add_filter("files", &borrowed);
            }
        }
        dialog
            .pick_file()
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_default()
    })
    .await
    .unwrap_or_default()
}

/// Port of `upload_file` (`POST /upload_file`).
pub(crate) async fn upload_file(mut multipart: Multipart) -> Response {
    let invalid = || {
        super::v2::error_response(
            crate::domain::error::AppError::new(
                crate::domain::error::ErrorCode::InvalidInput,
                "Invalid upload request",
            ),
            None,
        )
    };
    let mut pending = None;
    let mut info = None;
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(_) => return invalid(),
        };
        match field.name().unwrap_or("") {
            "file" if pending.is_none() => {
                let filename = field.file_name().unwrap_or("upload.bin").to_string();
                let path = match confined_path(&config_dir(), "user_uploads", &filename) {
                    Ok(path) => path,
                    Err(error) => return super::v2::error_response(error, None),
                };
                let bytes = match field.bytes().await {
                    Ok(bytes) => bytes,
                    Err(_) => return invalid(),
                };
                pending = Some((filename, path, bytes));
            }
            "info" if info.is_none() => {
                let value = match field.text().await {
                    Ok(value) if value.len() <= 128 => value,
                    _ => return invalid(),
                };
                info = Some(value);
            }
            _ => return invalid(),
        }
    }
    let Some((filename, path, bytes)) = pending else {
        return invalid();
    };
    match tokio::task::spawn_blocking(move || atomic_replace(&path, &bytes)).await {
        Ok(Ok(())) => (),
        _ => {
            return super::v2::error_response(
                crate::domain::error::AppError::new(
                    crate::domain::error::ErrorCode::PersistenceFailed,
                    "Cannot save upload",
                ),
                None,
            )
        }
    }

    if info.as_deref() == Some("background_image") {
        save_rotated_copy(
            &config_dir()
                .join("user_uploads")
                .join(&filename)
                .to_string_lossy(),
        );
    }

    log().success("File uploaded successfully");
    Json(json!({"success": true, "message": text(Some("downloaded_successfully"), None)}))
        .into_response()
}

/// Port of `get_config_file` (`GET /.config/<directory>/<filename>`).
pub(crate) async fn get_config_file(
    Path((directory, filename)): Path<(String, String)>,
) -> Response {
    if directory != "user_uploads" && directory != "themes" {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    let file_path = match confined_path(&config_dir(), &directory, &filename) {
        Ok(path) => path,
        Err(error) => return super::v2::error_response(error, None),
    };

    match tokio::task::spawn_blocking(move || crate::adapters::files::read_asset(&file_path)).await
    {
        Ok(Ok(bytes)) => match Response::builder()
            .header("content-type", "application/octet-stream")
            .header(
                "content-disposition",
                format!("attachment; filename=\"{filename}\""),
            )
            .body(Body::from(bytes))
        {
            Ok(response) => response.into_response(),
            Err(e) => internal_error(
                "An error occurred during a request",
                format!("Error: {e}"),
                None,
            ),
        },
        _ => (
            StatusCode::NOT_FOUND,
            format!("File '{filename}' not found."),
        )
            .into_response(),
    }
}
