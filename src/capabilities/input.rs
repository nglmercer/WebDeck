//! Persistent keyboard session for Wayland desktops, approved by the compositor.
use super::*;
use ashpd::desktop::{
    remote_desktop::{DeviceType, KeyState, RemoteDesktop, SelectDevicesOptions},
    Session,
};
use std::sync::OnceLock;
type Portal = (RemoteDesktop, Session<RemoteDesktop>);
static SESSION: Mutex<Option<Portal>> = Mutex::new(None);
static RUNTIME: OnceLock<std::io::Result<tokio::runtime::Runtime>> = OnceLock::new();
pub(super) fn wayland() -> bool {
    std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland")
}
fn symbol(s: &str) -> Result<i32> {
    let normalized = crate::keys::normalize(s)?;
    Ok(match normalized.as_str() {
        "ctrl" | "control" => 0xffe3,
        "shift" => 0xffe1,
        "alt" => 0xffe9,
        "meta" | "super" | "win" | "windows" => 0xffeb,
        "enter" | "return" => 0xff0d,
        "escape" | "esc" => 0xff1b,
        "tab" => 0xff09,
        "space" => 0x20,
        "backspace" => 0xff08,
        "delete" => 0xffff,
        "insert" => 0xff63,
        "capslock" => 0xffe5,
        "numlock" => 0xff7f,
        "scrolllock" => 0xff14,
        "printscreen" => 0xff61,
        "pause" => 0xff13,
        "numpad0" => 0xffb0,
        "numpad1" => 0xffb1,
        "numpad2" => 0xffb2,
        "numpad3" => 0xffb3,
        "numpad4" => 0xffb4,
        "numpad5" => 0xffb5,
        "numpad6" => 0xffb6,
        "numpad7" => 0xffb7,
        "numpad8" => 0xffb8,
        "numpad9" => 0xffb9,

        "home" => 0xff50,
        "end" => 0xff57,
        "pageup" => 0xff55,
        "pagedown" => 0xff56,
        "up" => 0xff52,
        "down" => 0xff54,
        "left" => 0xff51,
        "right" => 0xff53,
        f if f.starts_with('f') && f[1..].parse::<i32>().is_ok_and(|n| (1..=24).contains(&n)) => {
            0xffbd + f[1..].parse::<i32>().map_err(|_| Error::invalid())?
        }
        _ => {
            let mut chars = s.chars();
            let c = chars.next().ok_or_else(Error::invalid)?;
            if chars.next().is_some() {
                return Err(Error::invalid());
            }
            unicode(c)
        }
    })
}
fn unicode(c: char) -> i32 {
    if u32::from(c) <= 0xff {
        c as i32
    } else {
        0x01000000 | c as i32
    }
}
fn send(events: Vec<(i32, KeyState)>, deadline: Instant) -> Result<Value> {
    remaining(deadline)?;
    let rt = RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
        })
        .as_ref()
        .map_err(|_| {
            Error::new(
                ErrorCode::ExecutionFailed,
                "Cannot initialize host keyboard service",
            )
        })?;
    let mut slot = SESSION.try_lock().map_err(|_| {
        Error::new(
            ErrorCode::ExecutionFailed,
            "Keyboard is busy; check the host desktop permission dialog",
        )
    })?;
    let budget = remaining(deadline)?;
    let mut held = Vec::new();
    let mut pending: Option<Portal> = None;
    let mut result = rt.block_on(async {
        tokio::time::timeout(budget, async {
            if slot.is_none() {
                let portal = RemoteDesktop::with_connection(zbus::Connection::session().await.map_err(|_| Error::execution())?).await.map_err(|_| Error::execution())?;
                let session = portal.create_session(Default::default()).await.map_err(|_| Error::execution())?;
                pending = Some((portal,session));
                let (portal,session)=pending.as_ref().expect("pending portal");
                portal.select_devices(session, SelectDevicesOptions::default().set_devices(Some(DeviceType::Keyboard.into()))).await.map_err(|_| Error::execution())?.response().map_err(|_| Error::execution())?;
                portal.start(session, None, Default::default()).await.map_err(|_| Error::execution())?.response().map_err(|_| Error::new(ErrorCode::ExecutionFailed, "Keyboard access was declined; approve WebDeck in the desktop permission dialog"))?;
                *slot = pending.take();
            }
            let (portal, session) = slot.as_ref().expect("initialized portal");
            let mut result = Ok(json!({}));
            for (sym, state) in events {
                if state == KeyState::Pressed { held.push(sym); }
                if portal.notify_keyboard_keysym(session, sym, state, Default::default()).await.is_err() { result = Err(Error::execution()); break; }
                if state == KeyState::Released { held.retain(|s| *s != sym); }
            }
            result
        }).await.map_err(|_| Error::new(ErrorCode::ExecutionFailed, "Keyboard permission timed out; approve WebDeck on the host desktop and try again"))?
    });
    // Timeout cancels the send future. Keep held keys outside it so cleanup still runs.
    if !held.is_empty() {
        if let Some((portal, session)) = slot.as_ref() {
            let cleanup = rt.block_on(async {
                tokio::time::timeout(Duration::from_secs(1), async {
                    while let Some(sym) = held.last().copied() {
                        portal
                            .notify_keyboard_keysym(
                                session,
                                sym,
                                KeyState::Released,
                                Default::default(),
                            )
                            .await
                            .map_err(|_| Error::execution())?;
                        held.pop();
                    }
                    Ok::<(), Error>(())
                })
                .await
            });
            if !matches!(cleanup, Ok(Ok(()))) {
                result = Err(Error::new(
                    ErrorCode::ExecutionFailed,
                    "Keyboard release failed; portal session closed for recovery",
                ));
            }
        }
    }
    if result.is_err() {
        if let Some((_, session)) = slot.take().or_else(|| pending.take()) {
            rt.block_on(async {
                let _ = tokio::time::timeout(Duration::from_secs(1), session.close()).await;
            });
        }
    }
    result
}
pub(super) fn keys(keys: &[String], deadline: Instant) -> Result<Value> {
    let normalized = crate::keys::chord(keys)?;
    let symbols = normalized
        .iter()
        .map(|k| symbol(k))
        .collect::<Result<Vec<_>>>()?;
    send(
        symbols
            .iter()
            .map(|s| (*s, KeyState::Pressed))
            .chain(symbols.iter().rev().map(|s| (*s, KeyState::Released)))
            .collect(),
        deadline,
    )
}
pub(super) fn text(text: &str, enter: bool, deadline: Instant) -> Result<Value> {
    let mut events = Vec::new();
    for c in text.chars() {
        let sym = match c {
            '\n' => 0xff0d,
            '\t' => 0xff09,
            _ => unicode(c),
        };
        events.extend([(sym, KeyState::Pressed), (sym, KeyState::Released)]);
    }
    if enter {
        events.extend([(0xff0d, KeyState::Pressed), (0xff0d, KeyState::Released)]);
    }
    send(events, deadline)
}

fn remaining(deadline: Instant) -> Result<Duration> {
    let budget = deadline.saturating_duration_since(Instant::now());
    if budget.is_zero() {
        return Err(Error::new(
            ErrorCode::ExecutionFailed,
            "Execution budget exhausted",
        ));
    }
    Ok(budget.min(Duration::from_secs(20)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expired_input_is_rejected_before_runtime_or_portal_access() {
        let deadline = Instant::now() - Duration::from_secs(1);
        assert_eq!(
            keys(&["ctrl".into(), "c".into()], deadline)
                .unwrap_err()
                .code,
            ErrorCode::ExecutionFailed
        );
        assert_eq!(
            text("hello", true, deadline).unwrap_err().code,
            ErrorCode::ExecutionFailed
        );
        assert_eq!(
            remaining(Instant::now() + Duration::from_secs(60)).unwrap(),
            Duration::from_secs(20)
        );
    }
}
