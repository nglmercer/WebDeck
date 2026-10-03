use super::*;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{
    tungstenite::{protocol::WebSocketConfig, Message},
    MaybeTlsStream, WebSocketStream,
};
#[derive(Default)]
pub(super) struct WebSocketOwner(Mutex<std::collections::HashMap<String, Connection>>);
struct Connection {
    owner_id: String,
    runtime: tokio::runtime::Runtime,
    stream: WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
}
impl WebSocketOwner {
    pub fn call(&self, operation: &str, input: &Value, context: &Context) -> Result<Value> {
        context.check(Capability::Network)?;
        let remaining = context.remaining(Capability::Network, Duration::from_secs(30))?;
        let mut connections = self.0.lock().map_err(|_| Error::execution())?;
        if operation == "network.wsOpen" {
            if connections.len() >= 16 {
                return Err(Error::new(
                    ErrorCode::CapacityExhausted,
                    "WebSocket capacity exhausted",
                ));
            }
            let url = input["url"].as_str().ok_or_else(Error::invalid)?;
            let parsed = reqwest::Url::parse(url).map_err(|_| Error::invalid())?;
            if !matches!(parsed.scheme(), "ws" | "wss")
                || !parsed.username().is_empty()
                || parsed.password().is_some()
                || url.len() > 2048
            {
                return Err(Error::invalid());
            }
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| Error::execution())?;
            let stream = runtime.block_on(async {
                tokio::time::timeout(
                    remaining,
                    tokio_tungstenite::connect_async_with_config(
                        url,
                        Some(
                            WebSocketConfig::default()
                                .max_message_size(Some(65536))
                                .max_frame_size(Some(65536)),
                        ),
                        true,
                    ),
                )
                .await
                .map_err(|_| Error::execution())?
                .map(|(stream, _)| stream)
                .map_err(|_| Error::execution())
            })?;
            let id = domain::id()?;
            connections.insert(
                id.clone(),
                Connection {
                    owner_id: context.owner_id.clone(),
                    runtime,
                    stream,
                },
            );
            return Ok(json!(id));
        }
        let id = input["id"].as_str().ok_or_else(Error::invalid)?;
        let connection = connections
            .get_mut(id)
            .filter(|c| c.owner_id == context.owner_id)
            .ok_or_else(Error::invalid)?;
        let result = connection.runtime.block_on(async {
            tokio::time::timeout(remaining, async {
                match operation {
                    "network.wsSend" => {
                        let value = input.get("value").ok_or_else(Error::invalid)?;
                        let text = serde_json::to_string(value).map_err(|_| Error::invalid())?;
                        if text.len() > 65536 {
                            return Err(Error::invalid());
                        }
                        connection
                            .stream
                            .send(Message::Text(text.into()))
                            .await
                            .map_err(|_| Error::execution())?;
                        Ok(Value::Null)
                    }
                    "network.wsReceive" => loop {
                        let message = connection
                            .stream
                            .next()
                            .await
                            .ok_or_else(Error::execution)?
                            .map_err(|_| Error::execution())?;
                        match message {
                            Message::Text(text) => {
                                return serde_json::from_str(text.as_str())
                                    .map_err(|_| Error::execution())
                            }
                            Message::Close(_) => return Err(Error::execution()),
                            _ => {}
                        }
                    },
                    "network.wsClose" => {
                        let _ = connection.stream.close(None).await;
                        Ok(Value::Null)
                    }
                    _ => Err(Error::invalid()),
                }
            })
            .await
            .map_err(|_| Error::execution())?
        });
        if operation == "network.wsClose" || result.is_err() {
            connections.remove(id);
        }
        result
    }
    pub fn finish_root(&self, context: &Context) {
        self.0
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .retain(|_, connection| connection.owner_id != context.owner_id);
    }
    pub fn shutdown(&self) {
        self.0.lock().unwrap_or_else(|p| p.into_inner()).clear();
    }
}
