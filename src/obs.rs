//! Bounded, serialized OBS session shared by the trusted built-in and local admin.
use crate::{
    contracts::{ErrorCode, ObsSettings},
    domain::{Error, Result},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};
use tokio_tungstenite::{
    tungstenite::{protocol::WebSocketConfig, Message},
    MaybeTlsStream, WebSocketStream,
};
type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;
struct Session {
    settings: ObsSettings,
    runtime: Option<tokio::runtime::Runtime>,
    socket: Socket,
}
impl Drop for Session {
    fn drop(&mut self) {
        // Shutdown can originate on the application's async task. Blocking Runtime::drop
        // would panic there, so release this bounded I/O runtime without blocking it.
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_background();
        }
    }
}
static WAITERS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static REQUEST_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
struct Admission;
impl Drop for Admission {
    fn drop(&mut self) {
        WAITERS.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}
static SESSION: OnceLock<Mutex<Option<Session>>> = OnceLock::new();
fn failure(message: &str) -> Error {
    Error::new(ErrorCode::ExecutionFailed, message)
}
async fn receive(socket: &mut Socket) -> Result<Value> {
    for _ in 0..64 {
        match socket.next().await {
            Some(Ok(Message::Text(text))) => {
                return serde_json::from_str(&text).map_err(|_| failure("Invalid OBS response"))
            }
            Some(Ok(Message::Ping(data))) => socket
                .send(Message::Pong(data))
                .await
                .map_err(|_| failure("OBS disconnected"))?,
            Some(Ok(Message::Pong(_))) => {}
            _ => return Err(failure("OBS disconnected; reconnect on next request")),
        }
    }
    Err(failure("OBS message budget exhausted"))
}
async fn send(socket: &mut Socket, value: Value) -> Result<()> {
    socket
        .send(Message::Text(value.to_string().into()))
        .await
        .map_err(|_| failure("OBS send failed; operation outcome may be unknown"))
}
pub fn allowed(request: &str) -> bool {
    matches!(
        request,
        "GetVersion"
            | "GetSceneList"
            | "GetCurrentProgramScene"
            | "GetInputList"
            | "GetInputKindList"
            | "GetHotkeyList"
            | "GetSceneCollectionList"
            | "SetCurrentSceneCollection"
            | "CreateSceneCollection"
            | "CreateScene"
            | "RemoveScene"
            | "SetSceneName"
            | "SetCurrentProgramScene"
            | "CreateInput"
            | "RemoveInput"
            | "SetInputName"
            | "GetInputSettings"
            | "SetInputSettings"
            | "GetSceneItemList"
            | "CreateSceneItem"
            | "RemoveSceneItem"
            | "GetSceneItemTransform"
            | "SetSceneItemTransform"
            | "SetSceneItemEnabled"
            | "SetSceneItemIndex"
            | "GetSceneItemId"
            | "GetSceneItemEnabled"
            | "GetInputMute"
            | "SetInputMute"
            | "TriggerHotkeyByName"
            | "TriggerHotkeyByKeySequence"
            | "ToggleRecord"
            | "StartRecord"
            | "StopRecord"
            | "ToggleRecordPause"
            | "PauseRecord"
            | "ResumeRecord"
            | "ToggleStream"
            | "StartStream"
            | "StopStream"
            | "ToggleVirtualCam"
            | "StartVirtualCam"
            | "StopVirtualCam"
            | "GetStreamStatus"
            | "GetRecordStatus"
            | "GetVirtualCamStatus"
    )
}
pub fn request(
    settings: ObsSettings,
    request: &str,
    data: Value,
    deadline: Instant,
) -> Result<Value> {
    if !allowed(request) || !data.is_object() || data.to_string().len() > 65536 {
        return Err(Error::invalid());
    }
    let budget = deadline
        .saturating_duration_since(Instant::now())
        .min(Duration::from_secs(20));
    if budget.is_zero() {
        return Err(failure("OBS deadline expired"));
    }
    use std::sync::atomic::Ordering;
    WAITERS
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
            (count < 16).then_some(count + 1)
        })
        .map_err(|_| Error::new(ErrorCode::CapacityExhausted, "OBS request queue is full"))?;
    let _admission = Admission;
    let session_slot = SESSION.get_or_init(|| Mutex::new(None));
    let mut slot = loop {
        if let Ok(slot) = session_slot.try_lock() {
            break slot;
        }
        if Instant::now() >= deadline {
            return Err(failure("OBS deadline expired while waiting for connection"));
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    let budget = deadline
        .saturating_duration_since(Instant::now())
        .min(Duration::from_secs(20));
    if budget.is_zero() {
        return Err(failure("OBS deadline expired before connection"));
    }
    if slot.as_ref().is_some_and(|s| s.settings != settings) {
        *slot = None;
    }
    if slot.is_none() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| failure("Cannot initialize OBS transport"))?;
        let host = if settings.host.contains(':') {
            format!("[{}]", settings.host)
        } else {
            settings.host.clone()
        };
        let socket = runtime.block_on(async {
            tokio::time::timeout(budget, async {
                let (mut socket, _) = tokio_tungstenite::connect_async_with_config(
                    format!("ws://{host}:{}", settings.port),
                    Some(
                        WebSocketConfig::default()
                            .max_message_size(Some(1_048_576))
                            .max_frame_size(Some(1_048_576)),
                    ),
                    true,
                )
                .await
                .map_err(|_| failure("Cannot connect to configured OBS endpoint"))?;
                let hello = receive(&mut socket).await?;
                if hello["op"] != 0 || hello["d"]["rpcVersion"].as_u64().unwrap_or(0) < 1 {
                    return Err(failure("OBS WebSocket RPC 1 is required"));
                }
                let mut identify = json!({"rpcVersion":1,"eventSubscriptions":0});
                if let Some(auth) = hello["d"].get("authentication") {
                    let salt = auth["salt"].as_str().ok_or_else(Error::invalid)?;
                    let challenge = auth["challenge"].as_str().ok_or_else(Error::invalid)?;
                    let secret =
                        STANDARD.encode(Sha256::digest(format!("{}{salt}", settings.password)));
                    identify["authentication"] =
                        json!(STANDARD.encode(Sha256::digest(format!("{secret}{challenge}"))));
                }
                send(&mut socket, json!({"op":1,"d":identify})).await?;
                let identified = receive(&mut socket).await?;
                if identified["op"] != 2 || identified["d"]["negotiatedRpcVersion"] != 1 {
                    return Err(failure("OBS authentication or RPC negotiation failed"));
                }
                Ok(socket)
            })
            .await
            .map_err(|_| failure("OBS connection timed out"))?
        })?;
        *slot = Some(Session {
            settings,
            runtime: Some(runtime),
            socket,
        });
    }
    let session = slot.as_mut().expect("initialized");
    let id = format!("webdeck-{}", REQUEST_ID.fetch_add(1, Ordering::Relaxed));
    let remaining = deadline
        .saturating_duration_since(Instant::now())
        .min(Duration::from_secs(20));
    if remaining.is_zero() {
        return Err(failure("OBS deadline expired before request"));
    }
    let result = session
        .runtime
        .as_ref()
        .expect("OBS runtime")
        .block_on(async {
            tokio::time::timeout(remaining, async {
                send(
                    &mut session.socket,
                    json!({"op":6,"d":{"requestType":request,"requestId":id,"requestData":data}}),
                )
                .await?;
                for _ in 0..64 {
                    let response = receive(&mut session.socket).await?;
                    if response["op"] == 7 && response["d"]["requestId"] == id {
                        if response["d"]["requestStatus"]["result"] != true {
                            return Err(failure(&format!(
                                "OBS {request} failed with code {}; inspect OBS locally",
                                response["d"]["requestStatus"]["code"]
                            )));
                        }
                        return Ok(response["d"]
                            .get("responseData")
                            .cloned()
                            .unwrap_or(json!({})));
                    }
                }
                Err(failure("OBS response budget exhausted"))
            })
            .await
            .map_err(|_| {
                failure("OBS request timed out; mutation outcome unknown, rediscover before retry")
            })?
        });
    if result.is_err() {
        *slot = None;
    }
    result
}
pub fn shutdown_endpoint(settings: &ObsSettings) {
    if let Some(slot) = SESSION.get() {
        let mut slot = slot.lock().unwrap_or_else(|poison| poison.into_inner());
        if slot
            .as_ref()
            .is_some_and(|session| &session.settings == settings)
        {
            *slot = None;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reused_session_matches_unique_ids_and_reconnects_without_replaying() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let server = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                let listener=tokio::net::TcpListener::from_std(listener).unwrap();let mut ids=Vec::new();
                for count in [4,1] {
                    let (socket,_)=listener.accept().await.unwrap();let mut socket=tokio_tungstenite::accept_async(socket).await.unwrap();
                    socket.send(Message::Text(json!({"op":0,"d":{"rpcVersion":1}}).to_string().into())).await.unwrap();
                    assert!(socket.next().await.unwrap().unwrap().is_text());
                    socket.send(Message::Text(json!({"op":2,"d":{"negotiatedRpcVersion":1}}).to_string().into())).await.unwrap();
                    for _ in 0..count {
                        let value=socket.next().await.unwrap().unwrap().into_text().unwrap();let value:Value=serde_json::from_str(&value).unwrap();let id=value["d"]["requestId"].clone();ids.push(id.clone());
                        for (id,data) in [(json!("unrelated"),json!({"wrong":true})),(id,json!({"success":true}))] {socket.send(Message::Text(json!({"op":7,"d":{"requestId":id,"requestStatus":{"result":true},"responseData":data}}).to_string().into())).await.unwrap();}
                    }
                    socket.close(None).await.unwrap();
                }
                assert_ne!(ids[0],ids[1]);
            });
        });
        let settings = ObsSettings {
            host: "127.0.0.1".into(),
            port: port.into(),
            password: String::new(),
        };
        for _ in 0..2 {
            assert_eq!(
                request(
                    settings.clone(),
                    "GetVersion",
                    json!({}),
                    Instant::now() + Duration::from_secs(2)
                )
                .unwrap(),
                json!({"success":true})
            );
        }
        let parallel = (0..2)
            .map(|_| {
                let settings = settings.clone();
                std::thread::spawn(move || {
                    request(
                        settings,
                        "GetSceneList",
                        json!({}),
                        Instant::now() + Duration::from_secs(2),
                    )
                })
            })
            .collect::<Vec<_>>();
        for thread in parallel {
            thread.join().unwrap().unwrap();
        }
        assert!(request(
            settings.clone(),
            "GetVersion",
            json!({}),
            Instant::now() + Duration::from_secs(2)
        )
        .is_err());
        assert!(request(
            settings.clone(),
            "GetVersion",
            json!({}),
            Instant::now() + Duration::from_secs(2)
        )
        .is_ok());
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                shutdown_endpoint(&settings);
            });
        server.join().unwrap();
        assert!(!allowed("CallVendorRequest"));
    }
}
