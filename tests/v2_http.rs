use axum::{
    body::{to_bytes, Body},
    extract::ConnectInfo,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tower::ServiceExt;
use webdeck::{
    app::server::{app_router, AppState},
    application::executor::{CommandAdapter, CommandExecutor},
    domain::command::ParsedCommand,
};

struct Fake(AtomicUsize);
impl CommandAdapter for Fake {
    fn execute(&self, _: &ParsedCommand) -> Value {
        self.0.fetch_add(1, Ordering::SeqCst);
        json!({"success":true,"fake":"recorded"})
    }
}
fn dir_path() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("WEBDECK_CONFIG_DIR").unwrap()).join("config.json")
}
async fn request(
    state: &AppState,
    method: &str,
    path: &str,
    data: Value,
    peer: &str,
    token: Option<&str>,
    origin: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("host", "localhost:5000")
        .header("content-type", "application/json")
        .extension(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    if let Some(origin) = origin {
        builder = builder.header("origin", origin);
    }
    if method == "POST" && matches!(path, "/save_config" | "/create_folder") && token.is_some() {
        let snapshot = webdeck::application::config::shared(dir_path())
            .unwrap()
            .snapshot()
            .unwrap();
        builder = builder.header("x-webdeck-revision", snapshot.revision);
    }
    let response = app_router(state.clone())
        .oneshot(builder.body(Body::from(data.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

#[tokio::test]
async fn real_routes_enforce_identity_capabilities_revisions_origin_and_legacy_shapes_with_fake_effects(
) {
    let dir = std::env::temp_dir().join(format!("webdeck-v2-http-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::env::set_var("WEBDECK_CONFIG_DIR", &dir);
    std::env::set_var("WEBDECK_FAKE_EFFECTS", "1");
    std::fs::write(dir.join("config.json"),json!({"schema_version":2,"settings":{"language":"en_US","v2_security":"paired","allowed_networks":["127.0.0.1/32"]},"front":{"height":1,"width":2,"buttons":{"index":[{"message":"/key a"},{"VOID":"VOID"}]}}}).to_string()).unwrap();
    webdeck::app::utils::languages::init(
        "webdeck/translations",
        Some("webdeck/translations/misc"),
        "en_US",
    );
    let fake = Arc::new(Fake(AtomicUsize::new(0)));
    let state = AppState {
        local_ip: "192.168.1.2".into(),
        executor: Arc::new(CommandExecutor::new(fake.clone(), 4, 2)),
    };
    let remote = "192.168.1.3:3000";
    let local = "127.0.0.1:3000";
    assert_eq!(
        request(
            &state,
            "POST",
            "/api/v2/commands",
            json!({"message":"/key a"}),
            remote,
            None,
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &state,
            "POST",
            "/send-data",
            json!({"message":"/key a"}),
            remote,
            None,
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &state,
            "POST",
            "/api/v2/devices",
            json!({"name":"phone","capabilities":["read","input"],"ttl_seconds":60}),
            remote,
            None,
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(
            &state,
            "POST",
            "/api/v2/devices",
            json!({"name":"phone","capabilities":["read","input"],"ttl_seconds":60}),
            local,
            None,
            Some("https://evil.test")
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (status, grant) = request(
        &state,
        "POST",
        "/api/v2/devices",
        json!({"name":"phone","capabilities":["read","input"],"ttl_seconds":60}),
        local,
        None,
        Some("http://localhost:5000"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let token = grant["token"].as_str().unwrap();
    let (status, boot) = request(
        &state,
        "GET",
        "/api/v2/boot",
        json!({}),
        remote,
        Some(token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(boot["can_edit"], false);
    assert!(boot["config"]["settings"].get("obs").is_none());
    assert!(boot["config"]["settings"].get("spotify_api").is_none());
    assert_eq!(boot["commands"], json!({}));
    assert_eq!(
        request(
            &state,
            "GET",
            "/api/v2/settings/boot",
            json!({}),
            remote,
            Some(token),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(&state, "GET", "/api/boot", json!({}), local, None, None)
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(
            &state,
            "GET",
            "/api/v2/config",
            json!({}),
            local,
            Some("invalid"),
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &state,
            "POST",
            "/save_config",
            json!({"front":{"names_color":"#123456"}}),
            local,
            None,
            None
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        request(
            &state,
            "POST",
            "/api/v2/config",
            json!({"config":{}}),
            local,
            None,
            None
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(token.len(), 64);
    assert!(!std::fs::read_to_string(dir.join("devices.json"))
        .unwrap()
        .contains(token));
    let (status, result) = request(
        &state,
        "POST",
        "/api/v2/commands",
        json!({"message":"/key a","request_id":"test-1"}),
        remote,
        Some(token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["request_id"], "test-1");
    assert_eq!(result["state"], "completed");
    let (retired_status, _) = request(
        &state,
        "POST",
        "/send-data",
        json!({"message":"/key a"}),
        remote,
        Some(token),
        None,
    )
    .await;
    assert_eq!(retired_status, StatusCode::NOT_FOUND);
    let (status, result) = request(
        &state,
        "POST",
        "/api/v2/commands",
        json!({"message":"/PCshutdown"}),
        remote,
        Some(token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(result["code"], "forbidden");
    assert_eq!(fake.0.load(Ordering::SeqCst), 1);
    assert_eq!(
        request(
            &state,
            "POST",
            "/api/v2/commands",
            json!({"message":"/not-registered"}),
            local,
            None,
            None
        )
        .await
        .1["code"],
        "unknown_command"
    );
    assert_eq!(
        request(
            &state,
            "GET",
            "/api/v2/config",
            json!({}),
            remote,
            Some(token),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (_, snapshot) = request(
        &state,
        "GET",
        "/api/v2/config",
        json!({}),
        local,
        None,
        None,
    )
    .await;
    let mut config = snapshot["config"].clone();
    config["front"]["width"] = json!(3);
    let (status, saved) = request(
        &state,
        "POST",
        "/api/v2/config",
        json!({"revision":snapshot["revision"],"config":config}),
        local,
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        saved["config"]["front"]["buttons"]["index"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        request(
            &state,
            "POST",
            "/api/v2/config",
            json!({"revision":snapshot["revision"],"config":snapshot["config"]}),
            local,
            None,
            None
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    // Typed DTO rejections return the stable v2 envelope, not framework text.
    for invalid in [
        json!({"message": 42}),
        json!({"message":"/debug-send {}","extra":true}),
        json!({"message":"/debug-send {}","request_id":"bad id"}),
    ] {
        let (status, error) = request(
            &state,
            "POST",
            "/api/v2/commands",
            invalid,
            local,
            None,
            None,
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(error["api_version"], 2);
        assert_eq!(error["code"], "invalid_input");
        assert!(error["request_id"].is_string());
    }
    let (_, editor) = request(
        &state,
        "POST",
        "/api/v2/devices",
        json!({"name":"Editor","capabilities":["read","settings"],"ttl_seconds":60}),
        local,
        None,
        None,
    )
    .await;
    let editor_token = editor["token"].as_str().unwrap();
    for key in [
        "v2_security",
        "allowed_networks",
        "update_repo",
        "automatic_firewall_bypass",
    ] {
        let patch = json!({"settings":{key:"changed"}});
        let (status, error) = request(
            &state,
            "POST",
            "/save_config",
            patch,
            remote,
            Some(editor_token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(error["code"], "forbidden");
    }
    assert_eq!(
        request(
            &state,
            "POST",
            "/save_config",
            json!({"front":{"names_color":"#112233"}}),
            remote,
            Some(editor_token),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    for name in ["user_uploads", "themes"] {
        std::fs::create_dir_all(dir.join(name)).unwrap();
    }
    // Multipart duplicate/malformed fields are rejected before any file is published.
    for fields in ["--boundary\r\nContent-Disposition: form-data; name=\"file\"; filename=\"../escape\"\r\n\r\ninvalid\r\n--boundary--\r\n", "--boundary\r\nContent-Disposition: form-data; name=\"file\"; filename=\"first.txt\"\r\n\r\none\r\n--boundary\r\nContent-Disposition: form-data; name=\"file\"; filename=\"second.txt\"\r\n\r\ntwo\r\n--boundary--\r\n"] {
        let response = app_router(state.clone()).oneshot(Request::builder().method("POST").uri("/upload_file")
            .header("host", "localhost:5000").header("content-type","multipart/form-data; boundary=boundary").extension(ConnectInfo(local.parse::<SocketAddr>().unwrap())).body(Body::from(fields)).unwrap()).await.unwrap();
        assert_eq!(response.status(),StatusCode::BAD_REQUEST);
        assert_eq!(std::fs::read_dir(dir.join("user_uploads")).unwrap().count(),0);
    }
    let id = grant["device"]["id"].as_str().unwrap();
    assert_eq!(
        request(
            &state,
            "DELETE",
            &format!("/api/v2/devices/{id}"),
            json!({}),
            local,
            None,
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(
            &state,
            "POST",
            "/api/v2/commands",
            json!({"message":"/key a"}),
            remote,
            Some(token),
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(fake.0.load(Ordering::SeqCst), 1);
    let _ = std::fs::remove_dir_all(dir);
}
