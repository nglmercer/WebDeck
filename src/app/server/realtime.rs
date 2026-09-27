//! Realtime paths (extracted from `server.rs`).
//!
//! `POST /usage`, `POST /send-data`, and the SocketIO `/` namespace
//! (`connect`/`send`/`message_from_socket`).

use axum::response::{IntoResponse, Json, Response};
use serde_json::Value;

use super::internal_error;
use crate::app::buttons::{self, usage::get_usage};
use crate::app::utils::logger::log;

/// Port of `usage` (`POST /usage`).
pub(crate) async fn usage() -> Json<Value> {
    Json(get_usage(None, &[]))
}

/// SocketIO `/` namespace setup — port of the `@socketio.on(...)` handlers
/// in `app/server.py` (`connect`, `send`, `message_from_socket`).
pub(crate) fn socketio_layer(
    server_address: &str,
    server_port: u16,
) -> socketioxide::layer::SocketIoLayer {
    use socketioxide::extract::{Data, SocketRef};

    let (layer, io) = socketioxide::SocketIo::new_layer();
    let address = server_address.to_string();

    io.ns("/", move |socket: SocketRef| {
        let address = address.clone();
        async move {
            log().info(&format!("server connected at {address}:{server_port}"));

            socket.on(
                "send",
                |socket: SocketRef, Data::<Value>(data)| async move {
                    log().info(&format!("message received with : {data}"));
                    // Python `send(data, broadcast=True)` emits "message".
                    let _ = socket.broadcast().emit("message", &data).await;
                },
            );

            socket.on(
                "message_from_socket",
                |socket: SocketRef, Data::<Value>(data)| async move {
                    let message = data
                        .as_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| data.to_string());
                    log().info(&format!("Message from client: {message}"));
                    let owned = message.clone();
                    let result =
                        tokio::task::spawn_blocking(move || buttons::handle_command(&owned)).await;
                    if result.is_ok() {
                        // Python emits the ORIGINAL message, not the result.
                        let _ = socket.emit("json_data", &message);
                    }
                },
            );
        }
    });

    layer
}

/// Port of `send_data_route` (`POST /send-data`).
pub(crate) async fn send_data_route(Json(body): Json<Value>) -> Response {
    let message = body
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    // Command handling is blocking/sync (subprocesses, sleeps, sync HTTP for
    // Spotify/translate), like Python's gevent worker: run it off the runtime.
    let result = tokio::task::spawn_blocking(move || buttons::handle_command(&message)).await;
    match result {
        Ok(result) => {
            // NOTE: like Python's `send_data_route`, the HTTP path does not
            // emit SocketIO events; only `on_socket_message` emits `json_data`.
            Json(result).into_response()
        }
        Err(e) => internal_error(
            "An error occurred while handling a command",
            format!("Command task failed: {e}"),
            None,
        ),
    }
}
