use super::*;

pub(super) fn agent() -> Result<Enigo> {
    Enigo::new(&InputSettings::default()).map_err(|_| Error::execution())
}
pub(super) fn map_key(s: &str) -> Result<Key> {
    Ok(match s.to_lowercase().as_str() {
        "ctrl" | "control" => Key::Control,
        "shift" => Key::Shift,
        "alt" => Key::Alt,
        "meta" | "super" | "win" | "windows" => Key::Meta,
        "enter" | "return" => Key::Return,
        "escape" | "esc" => Key::Escape,
        "tab" => Key::Tab,
        "space" => Key::Space,
        "backspace" => Key::Backspace,
        "delete" => Key::Delete,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        "up" => Key::UpArrow,
        "down" => Key::DownArrow,
        "left" => Key::LeftArrow,
        "right" => Key::RightArrow,
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,
        _ => {
            if s.chars().count() != 1 {
                return Err(Error::invalid());
            }
            Key::Unicode(s.chars().next().ok_or_else(Error::invalid)?)
        }
    })
}
pub(super) static INPUT_LOCK: Mutex<()> = Mutex::new(());
pub(super) fn key(s: &[String], deadline: Instant) -> Result<Value> {
    #[cfg(target_os = "linux")]
    if input::wayland() {
        return input::keys(s, deadline);
    }
    let keys: Vec<_> = s.iter().map(|s| map_key(s)).collect::<Result<_>>()?;
    let mut a = agent()?;
    let mut pressed = Vec::new();
    let result = (|| {
        for k in keys {
            if Instant::now() >= deadline {
                return Err(Error::new(
                    ErrorCode::ExecutionFailed,
                    "Execution budget exhausted",
                ));
            }
            a.key(k, Direction::Press).map_err(|_| Error::execution())?;
            pressed.push(k);
        }
        Ok(json!({}))
    })();
    for k in pressed.into_iter().rev() {
        let _ = a.key(k, Direction::Release);
    }
    result
}
pub(super) static CLIPBOARD: Mutex<Option<arboard::Clipboard>> = Mutex::new(None);
pub(super) fn clipboard(s: &str) -> Result<Value> {
    let mut slot = CLIPBOARD.lock().unwrap_or_else(|p| p.into_inner());
    if slot.is_none() {
        *slot = Some(arboard::Clipboard::new().map_err(|_| {
            Error::new(
                ErrorCode::ExecutionFailed,
                "Cannot connect to the host clipboard",
            )
        })?);
    }
    slot.as_mut()
        .expect("initialized clipboard")
        .set_text(s.to_owned())
        .map_err(|_| Error::execution())?;
    Ok(json!({}))
}
