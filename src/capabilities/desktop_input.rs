use super::*;

pub(super) fn agent() -> Result<Enigo> {
    Enigo::new(&InputSettings::default()).map_err(|_| Error::execution())
}
pub(super) fn map_key(s: &str) -> Result<Key> {
    let normalized = crate::keys::normalize(s)?;
    Ok(match normalized.as_str() {
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
        "insert" => Key::Insert,
        "capslock" => Key::CapsLock,
        #[cfg(any(windows, target_os = "linux"))]
        "numlock" => Key::Numlock,
        #[cfg(windows)]
        "scrolllock" => Key::Scroll,
        #[cfg(target_os = "linux")]
        "scrolllock" => Key::ScrollLock,
        #[cfg(any(windows, target_os = "linux"))]
        "printscreen" => Key::PrintScr,
        #[cfg(any(windows, target_os = "linux"))]
        "pause" => Key::Pause,
        "numpad0" => Key::Numpad0,
        "numpad1" => Key::Numpad1,
        "numpad2" => Key::Numpad2,
        "numpad3" => Key::Numpad3,
        "numpad4" => Key::Numpad4,
        "numpad5" => Key::Numpad5,
        "numpad6" => Key::Numpad6,
        "numpad7" => Key::Numpad7,
        "numpad8" => Key::Numpad8,
        "numpad9" => Key::Numpad9,

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
        "f13" => Key::F13,
        "f14" => Key::F14,
        "f15" => Key::F15,
        "f16" => Key::F16,
        "f17" => Key::F17,
        "f18" => Key::F18,
        "f19" => Key::F19,
        "f20" => Key::F20,
        "f21" => Key::F21,
        "f22" => Key::F22,
        "f23" => Key::F23,
        "f24" => Key::F24,

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
    let normalized = crate::keys::chord(s)?;
    let s = &normalized;
    #[cfg(target_os = "linux")]
    if input::wayland() {
        return input::keys(s, deadline);
    }
    let keys: Vec<_> = s.iter().map(|s| map_key(s)).collect::<Result<_>>()?;
    let mut a = agent()?;
    perform_chord(keys, deadline, |key, direction| {
        a.key(key, direction).map_err(|_| Error::execution())
    })
}
fn perform_chord<K: Copy>(
    keys: Vec<K>,
    deadline: Instant,
    mut send: impl FnMut(K, Direction) -> Result<()>,
) -> Result<Value> {
    let mut pressed = Vec::new();
    let result = (|| {
        for key in keys {
            if Instant::now() >= deadline {
                return Err(Error::new(
                    ErrorCode::ExecutionFailed,
                    "Execution budget exhausted",
                ));
            }
            // Even a failing backend may have partially delivered a press.
            pressed.push(key);
            send(key, Direction::Press)?;
        }
        Ok(json!({}))
    })();
    let mut release_failed = false;
    for key in pressed.into_iter().rev() {
        release_failed |= send(key, Direction::Release).is_err();
    }
    if release_failed {
        return Err(Error::new(
            ErrorCode::ExecutionFailed,
            "Keyboard release failed; check host modifiers",
        ));
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_function_key_maps_and_failed_press_releases_in_reverse() {
        for n in 1..=24 {
            assert!(map_key(&format!("F{n}")).is_ok());
        }
        let mut events = Vec::new();
        let result = perform_chord(
            vec![1, 2, 3],
            Instant::now() + Duration::from_secs(1),
            |key, direction| {
                let press = matches!(direction, Direction::Press);
                events.push((key, press));
                if key == 2 && press {
                    Err(Error::execution())
                } else {
                    Ok(())
                }
            },
        );
        assert!(result.is_err());
        assert_eq!(events, [(1, true), (2, true), (2, false), (1, false)]);
        assert!(perform_chord(
            vec![1],
            Instant::now() - Duration::from_secs(1),
            |_, _| panic!("expired action sent a key")
        )
        .is_err());
    }
}
