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
    Ok(match s.to_lowercase().as_str() {
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
    let result = rt.block_on(async {
        tokio::time::timeout(budget, async {
            if slot.is_none() {
                let portal = RemoteDesktop::with_connection(zbus::Connection::session().await.map_err(|_| Error::execution())?).await.map_err(|_| Error::execution())?;
                let session = portal.create_session(Default::default()).await.map_err(|_| Error::execution())?;
                portal.select_devices(&session, SelectDevicesOptions::default().set_devices(Some(DeviceType::Keyboard.into()))).await.map_err(|_| Error::execution())?.response().map_err(|_| Error::execution())?;
                portal.start(&session, None, Default::default()).await.map_err(|_| Error::execution())?.response().map_err(|_| Error::new(ErrorCode::ExecutionFailed, "Keyboard access was declined; approve WebDeck in the desktop permission dialog"))?;
                *slot = Some((portal, session));
            }
            let (portal, session) = slot.as_ref().expect("initialized portal");
            let mut held = Vec::new();
            let mut result = Ok(json!({}));
            for (sym, state) in events {
                if portal.notify_keyboard_keysym(session, sym, state, Default::default()).await.is_err() { result = Err(Error::execution()); break; }
                if state == KeyState::Pressed { held.push(sym); } else { held.retain(|s| *s != sym); }
            }
            for sym in held.into_iter().rev() { let _ = portal.notify_keyboard_keysym(session, sym, KeyState::Released, Default::default()).await; }
            result
        }).await.map_err(|_| Error::new(ErrorCode::ExecutionFailed, "Keyboard permission timed out; approve WebDeck on the host desktop and try again"))?
    });
    if result.is_err() {
        if let Some((_, session)) = slot.take() {
            rt.block_on(async {
                let _ = tokio::time::timeout(Duration::from_secs(1), session.close()).await;
            });
        }
    }
    result
}
pub(super) fn keys(keys: &[String], deadline: Instant) -> Result<Value> {
    let symbols = keys.iter().map(|k| symbol(k)).collect::<Result<Vec<_>>>()?;
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
