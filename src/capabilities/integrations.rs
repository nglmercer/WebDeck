use super::*;
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
            principal: None,
            owner_id: "test-root".into(),
            capabilities: vec![Capability::Audio],
            deadline: Instant::now() - Duration::from_secs(1),
            depth: 0,
        };
        assert_eq!(
            media("play-pause", &expired).unwrap_err().code,
            ErrorCode::ExecutionFailed
        );
        let denied = Context {
            principal: None,
            owner_id: "test-root".into(),
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
