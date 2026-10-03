use super::*;
impl Native {
    pub(super) fn obs(&self, action: &str, target: &str, context: &Context) -> Result<Value> {
        context.check(Capability::Network)?;
        let c = self.config.last_valid().config.settings.obs;
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| Error::execution())?;
        rt.block_on(async {
            tokio::time::timeout(
                context.remaining(Capability::Network, Duration::from_secs(30))?,
                async {
                    let client =
                        obws::Client::connect(c.host, c.port as u16, Some(c.password.as_str()))
                            .await
                            .map_err(|_| Error::execution())?;
                    context.check(Capability::Network)?;
                    let result = match action {
                        "toggle_recording" => client.recording().toggle().await.map(|_| ()),
                        "start_recording" => client.recording().start().await,
                        "stop_recording" => client.recording().stop().await.map(|_| ()),
                        "toggle_recording_pause" => {
                            client.recording().toggle_pause().await.map(|_| ())
                        }
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
                },
            )
            .await
            .map_err(|_| Error::execution())?
        })
    }
    pub(super) fn spotify(
        &self,
        action: &str,
        target: &str,
        change: &VolumeChange,
        context: &Context,
    ) -> Result<Value> {
        use rspotify::{prelude::*, AuthCodeSpotify, Credentials, OAuth, Token};
        context.check(Capability::Network)?;
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
            tokio::time::timeout(
                context.remaining(Capability::Network, Duration::from_secs(30))?,
                async {
                    client.auto_reauth().await.map_err(|_| Error::execution())?;
                    context.check(Capability::Network)?;
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
                },
            )
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
    let needs_track = !matches!(action, "volume" | "play_song" | "play_playlist");
    let current = if needs_track {
        spotify_get(c, "me/player/currently-playing".into(), Default::default()).await?
    } else {
        Value::Null
    };
    let track = current["item"]["id"].as_str().unwrap_or("");
    let uri = current["item"]["uri"].as_str().unwrap_or("");
    if needs_track && (track.is_empty() || uri.is_empty()) {
        return Err(Error::new(
            ErrorCode::ExecutionFailed,
            "This Spotify action needs a current track",
        ));
    }
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

#[cfg(target_os = "linux")]
pub(super) fn media(action: &str, context: &Context) -> Result<Value> {
    let method = match action {
        "play-pause" => "PlayPause",
        "next" => "Next",
        "previous" => "Previous",
        _ => return Err(Error::invalid()),
    };
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| Error::execution())?;
    rt.block_on(async {
        tokio::time::timeout(
            context.remaining(Capability::Audio, Duration::from_secs(5))?,
            async {
                let connection = zbus::Connection::session()
                    .await
                    .map_err(|_| Error::execution())?;
                let bus = zbus::fdo::DBusProxy::new(&connection)
                    .await
                    .map_err(|_| Error::execution())?;
                let names = bus.list_names().await.map_err(|_| Error::execution())?;
                let mut players = names
                    .into_iter()
                    .filter(|n| n.as_str().starts_with("org.mpris.MediaPlayer2."))
                    .collect::<Vec<_>>();
                players.sort_by(|a, b| a.as_str().cmp(b.as_str()));
                let mut chosen = None;
                for name in players {
                    let player = zbus::Proxy::new(
                        &connection,
                        name,
                        "/org/mpris/MediaPlayer2",
                        "org.mpris.MediaPlayer2.Player",
                    )
                    .await
                    .map_err(|_| Error::execution())?;
                    let playing = player
                        .get_property::<String>("PlaybackStatus")
                        .await
                        .is_ok_and(|s| s == "Playing");
                    if chosen.is_none() || playing {
                        chosen = Some(player);
                    }
                    if playing {
                        break;
                    }
                }
                let player = chosen.ok_or_else(|| {
                    Error::new(
                        ErrorCode::ExecutionFailed,
                        "No media player is available; open a player on the host desktop",
                    )
                })?;
                context.check(Capability::Audio)?;
                player.call::<_, _, ()>(method, &()).await.map_err(|_| {
                    Error::new(
                        ErrorCode::ExecutionFailed,
                        "The media player rejected the action",
                    )
                })?;
                Ok(json!({}))
            },
        )
        .await
        .map_err(|_| {
            Error::new(
                ErrorCode::ExecutionFailed,
                "Media player did not respond within the execution budget",
            )
        })?
    })
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn media_rejects_expired_or_denied_context_before_bus_access() {
        let expired = Context {
            capabilities: vec![Capability::Audio],
            deadline: Instant::now() - Duration::from_secs(1),
            depth: 0,
        };
        assert_eq!(
            media("play-pause", &expired).unwrap_err().code,
            ErrorCode::ExecutionFailed
        );
        let denied = Context {
            capabilities: vec![Capability::Read],
            deadline: Instant::now() + Duration::from_secs(30),
            depth: 0,
        };
        assert_eq!(
            media("next", &denied).unwrap_err().code,
            ErrorCode::Forbidden
        );
    }
}
