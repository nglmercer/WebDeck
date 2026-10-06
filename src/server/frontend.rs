use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};

#[cfg(webdeck_embedded_frontend)]
use axum::extract::Path;
#[cfg(not(webdeck_embedded_frontend))]
use axum::{
    http::{header, HeaderValue},
    middleware,
};

#[cfg(webdeck_embedded_frontend)]
include!(concat!(env!("OUT_DIR"), "/frontend.rs"));

#[cfg(webdeck_embedded_frontend)]
pub(super) fn translation(name: &str) -> Option<String> {
    embedded_file(&format!("/translations/{name}.lang"))
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .map(str::to_owned)
}

pub(super) fn routes<S: Clone + Send + Sync + 'static>() -> Router<S> {
    let router = Router::new().route("/", get(home));
    #[cfg(webdeck_embedded_frontend)]
    {
        router
            .route("/assets/{*path}", get(bundled_asset))
            .route("/static/{*path}", get(bundled_static))
    }
    #[cfg(not(webdeck_embedded_frontend))]
    {
        use tower_http::services::ServeDir;
        router
            .nest_service(
                "/assets",
                ServeDir::new(disk_root("frontend/dist").join("assets")),
            )
            .nest_service("/static", ServeDir::new(disk_root("static")))
            .layer(middleware::map_response(no_cache))
    }
}

#[cfg(not(webdeck_embedded_frontend))]
pub(super) fn disk_root(relative: &str) -> std::path::PathBuf {
    let local = std::path::PathBuf::from(relative);
    if local.is_dir() {
        local
    } else {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
    }
}

#[cfg(not(webdeck_embedded_frontend))]
async fn no_cache(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

pub(super) async fn home() -> Response {
    #[cfg(webdeck_embedded_frontend)]
    {
        html_response(embedded_file("/").map(|bytes| bytes.to_vec()))
    }
    #[cfg(not(webdeck_embedded_frontend))]
    {
        disk_home(&disk_root("frontend/dist").join("index.html")).await
    }
}

#[cfg(not(webdeck_embedded_frontend))]
async fn disk_home(path: &std::path::Path) -> Response {
    html_response(tokio::fs::read(path).await.ok())
}

fn html_response(bytes: Option<Vec<u8>>) -> Response {
    match bytes {
        Some(bytes) if super::frontend_matches_contract(&bytes) => (
            [("content-type", "text/html; charset=utf-8"), ("cache-control", "no-store")], bytes,
        ).into_response(),
        Some(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            [("cache-control", "no-store")],
            "Frontend build is out of date. Run npm ci --prefix frontend && npm run build --prefix frontend, then reload WebDeck. Your configuration is unchanged.",
        ).into_response(),
        None => (StatusCode::SERVICE_UNAVAILABLE, [("cache-control", "no-store")], "Build the frontend before starting WebDeck").into_response(),
    }
}

#[cfg(webdeck_embedded_frontend)]
async fn bundled_asset(Path(path): Path<String>) -> Response {
    bundled_response(&format!("/assets/{path}"))
}

#[cfg(webdeck_embedded_frontend)]
async fn bundled_static(Path(path): Path<String>) -> Response {
    bundled_response(&format!("/static/{path}"))
}

#[cfg(webdeck_embedded_frontend)]
fn bundled_response(path: &str) -> Response {
    let Some(bytes) = embedded_file(path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let content_type = match path.rsplit('.').next().unwrap_or("") {
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    };
    (
        [
            ("content-type", content_type),
            ("cache-control", "no-store"),
        ],
        bytes,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn frontend_routes_serve_assets_and_reject_missing_paths() {
        let html = home().await;
        assert_eq!(html.status(), StatusCode::OK);
        assert_eq!(html.headers()["cache-control"], "no-store");
        let bytes = to_bytes(html.into_body(), usize::MAX).await.unwrap();
        let html = String::from_utf8(bytes.to_vec()).unwrap();
        let assets: Vec<_> = html
            .split('"')
            .filter(|value| value.starts_with("/assets/"))
            .collect();
        assert!(!assets.is_empty());
        for path in assets.into_iter().chain(["/static/icons/icon.ico"]) {
            let response = routes::<()>()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{path}");
            assert!(response.headers().contains_key("content-type"));
            assert_eq!(response.headers()["cache-control"], "no-store");
            assert!(!to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap()
                .is_empty());
        }
        for path in ["/assets/missing.js", "/assets/../index.html"] {
            let response = routes::<()>()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        }
    }

    #[cfg(not(webdeck_embedded_frontend))]
    #[tokio::test]
    async fn debug_reads_updated_html_without_recompiling() {
        let path = std::env::temp_dir().join(format!(
            "webdeck-frontend-{}.html",
            crate::domain::id().unwrap()
        ));
        let original = tokio::fs::read(disk_root("frontend/dist").join("index.html"))
            .await
            .unwrap();
        tokio::fs::write(&path, &original).await.unwrap();
        assert_eq!(disk_home(&path).await.status(), StatusCode::OK);
        let mut updated = original.clone();
        updated.extend_from_slice(b"<!-- rebuilt UI -->");
        tokio::fs::write(&path, &updated).await.unwrap();
        let response = disk_home(&path).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap()
                .as_ref(),
            updated
        );
        tokio::fs::write(&path, b"outdated UI").await.unwrap();
        assert_eq!(
            disk_home(&path).await.status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        tokio::fs::remove_file(path).await.unwrap();
    }
}
