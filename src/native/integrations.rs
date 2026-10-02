use super::*;
impl Native {
    pub(super) fn obs(&self, action: &str, target: &str) -> Result<Value> {
        let c = self.config.last_valid().config.settings.obs;
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| Error::execution())?;
        rt.block_on(async {
            tokio::time::timeout(Duration::from_secs(30), async {
                let client =
                    obws::Client::connect(c.host, c.port as u16, Some(c.password.as_str()))
                        .await
                        .map_err(|_| Error::execution())?;
                let result = match action {
                    "toggle_recording" => client.recording().toggle().await.map(|_| ()),
                    "start_recording" => client.recording().start().await,
                    "stop_recording" => client.recording().stop().await.map(|_| ()),
                    "toggle_recording_pause" => client.recording().toggle_pause().await.map(|_| ()),
                    "pause_recording" => client.recording().pause().await,
                    "resume_recording" => client.recording().resume().await,
                    "toggle_stream" => client.streaming().toggle().await.map(|_| ()),
                    "start_stream" => client.streaming().start().await,
                    "stop_stream" => client.streaming().stop().await,
                    "toggle_virtual_camera" => client.virtual_cam().toggle().await.map(|_| ()),
                    "start_virtual_camera" => client.virtual_cam().start().await,
                    "stop_virtual_camera" => client.virtual_cam().stop().await,
                    "scene" => client.scenes().set_current_program_scene(target).await,
                    "hotkey" => {
                        client
                            .hotkeys()
                            .trigger_by_sequence(
                                target,
                                obws::requests::hotkeys::KeyModifiers::default(),
                            )
                            .await
                    }
                    _ => return Err(Error::invalid()),
                };
                result.map_err(|_| Error::execution())?;
                Ok(json!({}))
            })
            .await
            .map_err(|_| Error::execution())?
        })
    }
    pub(super) fn spotify(
        &self,
        action: &str,
        target: &str,
        change: &VolumeChange,
    ) -> Result<Value> {
        use rspotify::{prelude::*, AuthCodeSpotify, Credentials, OAuth, Token};
        let s = self.config.last_valid().config.settings.spotify;
        let token_path = self.assets.root.join("spotify-token.json");
        let token: Token = serde_json::from_slice(&fs::read(&token_path).map_err(|_| {
            Error::new(
                ErrorCode::Unauthorized,
                "Connect Spotify from local settings first",
            )
        })?)
        .map_err(|_| Error::execution())?;
        let oauth = OAuth {
            redirect_uri: s.redirect_uri,
            scopes: rspotify::scopes!(
                "user-read-playback-state",
                "user-modify-playback-state",
                "user-library-modify",
                "playlist-modify-public",
                "playlist-modify-private",
                "user-follow-modify",
                "user-follow-read",
                "user-library-read"
            ),
            ..Default::default()
        };
        let client = AuthCodeSpotify::from_token_with_config(
            token,
            Credentials::new(&s.client_id, &s.client_secret),
            oauth,
            Default::default(),
        );
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| Error::execution())?;
        rt.block_on(async {
            tokio::time::timeout(Duration::from_secs(30), async {
                client.auto_reauth().await.map_err(|_| Error::execution())?;
                spotify_action(&client, action, target, change).await?;
                if let Ok(t) = client.token.lock().await {
                    if let Some(t) = t.as_ref() {
                        crate::storage::atomic_replace(
                            &token_path,
                            &serde_json::to_vec(t).map_err(|_| Error::execution())?,
                        )?;
                    }
                }
                Ok(json!({}))
            })
            .await
            .map_err(|_| Error::execution())?
        })
    }
}
async fn spotify_action(
    c: &rspotify::AuthCodeSpotify,
    action: &str,
    target: &str,
    change: &VolumeChange,
) -> Result<()> {
    use rspotify::prelude::*;
    let current = spotify_get(c, "me/player/currently-playing".into(), Default::default()).await?;
    let track = current["item"]["id"]
        .as_str()
        .ok_or_else(Error::execution)?;
    let uri = current["item"]["uri"]
        .as_str()
        .ok_or_else(Error::execution)?;
    match action {
        "save_song" => {
            c.api_put("me/tracks", &json!({"ids":[track]}))
                .await
                .map_err(|_| Error::execution())?;
        }
        "save_album" => {
            let album = current["item"]["album"]["id"]
                .as_str()
                .ok_or_else(Error::execution)?;
            c.api_put("me/albums", &json!({"ids":[album]}))
                .await
                .map_err(|_| Error::execution())?;
        }
        "volume" => {
            let playback = spotify_get(c, "me/player".into(), Default::default()).await?;
            let p = percent(
                change,
                playback["device"]["volume_percent"].as_i64().unwrap_or(0),
            );
            c.api_put(
                &format!("me/player/volume?volume_percent={p}"),
                &Value::Null,
            )
            .await
            .map_err(|_| Error::execution())?;
        }
        "play_song" | "play_playlist" => {
            let kind = if action == "play_song" {
                "track"
            } else {
                "playlist"
            };
            let items = spotify_get(
                c,
                "search".into(),
                std::collections::HashMap::from([("q", target), ("type", kind), ("limit", "1")]),
            )
            .await?;
            let u = items[format!("{kind}s")]["items"][0]["uri"]
                .as_str()
                .ok_or_else(Error::execution)?;
            let payload = if kind == "track" {
                json!({"uris":[u]})
            } else {
                json!({"context_uri":u})
            };
            c.api_put("me/player/play", &payload)
                .await
                .map_err(|_| Error::execution())?;
        }
        "add_to_playlist" | "remove_from_playlist" | "toggle_playlist" => {
            let list = spotify_get(
                c,
                "search".into(),
                std::collections::HashMap::from([
                    ("q", target),
                    ("type", "playlist"),
                    ("limit", "1"),
                ]),
            )
            .await?;
            let id = list["playlists"]["items"][0]["id"]
                .as_str()
                .filter(|s| s.bytes().all(|b| b.is_ascii_alphanumeric()))
                .ok_or_else(Error::execution)?;
            let endpoint = format!("playlists/{id}/tracks");
            let remove = if action == "toggle_playlist" {
                let list =
                    spotify_get(c, format!("{endpoint}?limit=100"), Default::default()).await?;
                list["items"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|t| t["track"]["id"] == track))
            } else {
                action == "remove_from_playlist"
            };
            if remove {
                c.api_delete(&endpoint, &json!({"tracks":[{"uri":uri}]}))
                    .await
                    .map_err(|_| Error::execution())?;
            } else {
                c.api_post(&endpoint, &json!({"uris":[uri]}))
                    .await
                    .map_err(|_| Error::execution())?;
            }
        }
        "follow_artist" | "unfollow_artist" | "toggle_artist" => {
            let id = current["item"]["artists"][0]["id"]
                .as_str()
                .ok_or_else(Error::execution)?;
            let follow = if action == "toggle_artist" {
                let status = spotify_get(
                    c,
                    "me/following/contains".into(),
                    std::collections::HashMap::from([("type", "artist"), ("ids", id)]),
                )
                .await?;
                status[0] != true
            } else {
                action == "follow_artist"
            };
            let path = "me/following?type=artist";
            if follow {
                c.api_put(path, &json!({"ids":[id]}))
                    .await
                    .map_err(|_| Error::execution())?;
            } else {
                c.api_delete(path, &json!({"ids":[id]}))
                    .await
                    .map_err(|_| Error::execution())?;
            }
        }
        _ => return Err(Error::invalid()),
    }
    Ok(())
}

async fn spotify_get(
    c: &rspotify::AuthCodeSpotify,
    path: String,
    q: std::collections::HashMap<&str, &str>,
) -> Result<Value> {
    use rspotify::prelude::*;
    let s = c.api_get(&path, &q).await.map_err(|_| Error::execution())?;
    serde_json::from_str(&s).map_err(|_| Error::execution())
}
