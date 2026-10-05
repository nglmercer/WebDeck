use super::*;
use crate::runtime::capabilities::{required_capability, CapabilityHost};
pub struct FakePlatform {
    platform: Arc<Platform>,
    websocket_turn: Mutex<u8>,
}
impl FakePlatform {
    pub fn new(platform: Arc<Platform>) -> Self {
        Self {
            platform,
            websocket_turn: Mutex::new(0),
        }
    }
}
impl CapabilityHost for FakePlatform {
    fn call(&self, operation: &str, input: &Value, context: &Context) -> Result<Value> {
        context.check(required_capability(operation)?)?;
        match operation {
            "storage.pluginGet"
            | "storage.pluginSet"
            | "storage.source"
            | "storage.button"
            | "crypto.sha256Base64"
            | "crypto.base64" => self.platform.call(operation, input, context),
            "secrets.integration" => {
                let id = input["id"].as_str().ok_or_else(Error::invalid)?;
                if context.principal.as_deref() != Some(&format!("builtin.{id}")) {
                    return Err(Error::new(ErrorCode::Forbidden, "Secret scope denied"));
                }
                if id == "obs" {
                    Ok(json!({"host":"127.0.0.1","port":4455,"password":""}))
                } else if id == "spotify" {
                    Ok(
                        json!({"clientId":"fake","clientSecret":"fake","token":{"access_token":"fake","expires_at_ms":4070908800000u64}}),
                    )
                } else {
                    Err(Error::invalid())
                }
            }
            "network.fetch" => Ok(json!({"status":200,"body":"","truncated":false,"headers":{}})),
            "network.wsOpen" => {
                *self.websocket_turn.lock().unwrap() = 0;
                Ok(json!("fake-connection"))
            }
            "network.wsReceive" => {
                let mut turn = self.websocket_turn.lock().unwrap();
                *turn += 1;
                Ok(match *turn {
                    1 => json!({"op":0,"d":{}}),
                    2 => json!({"op":2}),
                    _ => {
                        json!({"op":7,"d":{"requestId":"webdeck","requestStatus":{"result":true},"responseData":{}}})
                    }
                })
            }
            "network.wsSend" | "network.wsClose" | "secrets.saveSpotifyToken" => Ok(Value::Null),
            _ => Ok(json!({"effect":"simulated"})),
        }
    }
    fn finish_root(&self, context: &Context) {
        self.platform.finish_root(context);
    }
    fn shutdown(&self) {
        self.platform.shutdown();
    }
}
