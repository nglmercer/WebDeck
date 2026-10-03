use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
impl Platform {
    pub(super) fn secret(&self, input: &Value, context: &Context) -> Result<Value> {
        context.check(Capability::Network)?;
        let id = input["id"].as_str().ok_or_else(Error::invalid)?;
        if context.principal.as_deref() != Some(&format!("builtin.{id}")) {
            return Err(Error::new(ErrorCode::Forbidden, "Secret scope denied"));
        }
        let settings = self.config.last_valid().config.settings;
        match id {
            "obs" => serde_json::to_value(settings.obs).map_err(|_| Error::execution()),
            "spotify" => {
                let path = self.assets.root.join("spotify-token.json");
                let bytes = fs::read(path).map_err(|_| {
                    Error::new(
                        ErrorCode::Unauthorized,
                        "Connect Spotify from local settings first",
                    )
                })?;
                if bytes.len() > 65536 {
                    return Err(Error::invalid());
                }
                let token: Value =
                    serde_json::from_slice(&bytes).map_err(|_| Error::execution())?;
                Ok(
                    json!({"clientId":settings.spotify.client_id,"clientSecret":settings.spotify.client_secret,"token":token}),
                )
            }
            _ => Err(Error::invalid()),
        }
    }
    pub(super) fn save_token(&self, input: &Value, context: &Context) -> Result<Value> {
        if context.principal.as_deref() != Some("builtin.spotify") {
            return Err(Error::new(ErrorCode::Forbidden, "Secret scope denied"));
        }
        let token = input
            .get("token")
            .filter(|v| v.is_object())
            .ok_or_else(Error::invalid)?;
        let bytes = serde_json::to_vec(token).map_err(|_| Error::invalid())?;
        if bytes.len() > 65536 {
            return Err(Error::invalid());
        }
        crate::storage::atomic_replace(&self.assets.root.join("spotify-token.json"), &bytes)?;
        Ok(Value::Null)
    }
}
pub(super) fn crypto(operation: &str, input: &Value) -> Result<Value> {
    let text = input["text"]
        .as_str()
        .filter(|s| s.len() <= 65536)
        .ok_or_else(Error::invalid)?;
    match operation {
        "crypto.sha256Base64" => {
            use sha2::{Digest, Sha256};
            Ok(json!(STANDARD.encode(Sha256::digest(text.as_bytes()))))
        }
        "crypto.base64" => Ok(json!(STANDARD.encode(text.as_bytes()))),
        _ => Err(Error::invalid()),
    }
}
