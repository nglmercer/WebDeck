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
    let mut saved_name: Option<String> = None;
    let mut info: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();
        if field_name == "file" {
            let filename = field.file_name().unwrap_or("upload.bin").to_string();
            let filename = std::path::Path::new(&filename)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("upload.bin")
                .to_string();
            match field.bytes().await {
                Ok(bytes) => {
                    let save_path = format!(".config/user_uploads/{filename}");
                    match std::fs::write(&save_path, &bytes) {
                        Ok(()) => saved_name = Some(filename),
                        Err(e) => {
                            return internal_error(
                                "An error occurred during a request",
                                format!("Cannot save upload: {e}"),
                                None,
                            );
                        }
                    }
                }
                Err(e) => {
                    return internal_error(
                        "An error occurred during a request",
                        format!("Cannot read upload: {e}"),
                        None,
                    );
                }
            }
        } else if field_name == "info" {
            info = field.text().await.ok();
        }
    }

    let Some(filename) = saved_name else {
        log().error("No files were found in the request.");
        return Json(
            json!({"success": false, "message": text(Some("no_files_found_error"), None)}),
        )
        .into_response();
    };

    if info.as_deref() == Some("background_image") {
        save_rotated_copy(&format!(".config/user_uploads/{filename}"));
    }

    log().success(&format!("File '{filename}' uploaded successfully"));
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

    let filename = std::path::Path::new(&filename)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let file_path = format!(".config/{directory}/{filename}");

    match tokio::fs::read(&file_path).await {
        Ok(bytes) => match Response::builder()
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
        Err(_) => (
            StatusCode::NOT_FOUND,
            format!("File '{filename}' not found."),
        )
            .into_response(),
    }
}
