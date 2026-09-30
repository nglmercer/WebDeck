//! Input/clipboard backends (extracted from `commands.rs`).
//!
//! `pyautogui.press/hotkey` → enigo `key()` clicks; `keyboard.write` →
//! enigo `text()`; `pyperclip.copy` → arboard. pyautogui key names map to
//! enigo `Key` below (`Key::Unicode` covers single characters everywhere).

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

use crate::app::utils::logger::log;

pub(crate) fn enigo_agent() -> Result<Enigo, String> {
    Enigo::new(&Settings::default()).map_err(|e| e.to_string())
}

/// Map a pyautogui/`keyboard` key name to enigo. Returns `None` when the
/// name is better sent as typed text (single char) — handled by callers.
pub(crate) fn map_key(name: &str) -> Option<Key> {
    let lower = name.to_lowercase();
    // Letters/digits exist as Key variants on Windows only; elsewhere they
    // are typed as Unicode text (same visible effect as pyautogui.press).
    #[cfg(windows)]
    {
        if lower.len() == 1 {
            let ch = lower.chars().next().unwrap_or('\0');
            let key = match ch {
                'a' => Key::A,
                'b' => Key::B,
                'c' => Key::C,
                'd' => Key::D,
                'e' => Key::E,
                'f' => Key::F,
                'g' => Key::G,
                'h' => Key::H,
                'i' => Key::I,
                'j' => Key::J,
                'k' => Key::K,
                'l' => Key::L,
                'm' => Key::M,
                'n' => Key::N,
                'o' => Key::O,
                'p' => Key::P,
                'q' => Key::Q,
                'r' => Key::R,
                's' => Key::S,
                't' => Key::T,
                'u' => Key::U,
                'v' => Key::V,
                'w' => Key::W,
                'x' => Key::X,
                'y' => Key::Y,
                'z' => Key::Z,
                '0' => Key::Num0,
                '1' => Key::Num1,
                '2' => Key::Num2,
                '3' => Key::Num3,
                '4' => Key::Num4,
                '5' => Key::Num5,
                '6' => Key::Num6,
                '7' => Key::Num7,
                '8' => Key::Num8,
                '9' => Key::Num9,
                _ => return None,
            };
            return Some(key);
        }
    }
    match lower.as_str() {
        "ctrl" | "control" | "lctrl" => Some(Key::LControl),
        "rctrl" => Some(Key::RControl),
        "shift" | "lshift" => Some(Key::LShift),
        "rshift" => Some(Key::RShift),
        "alt" | "lalt" => Some(Key::Alt),
        "enter" | "return" => Some(Key::Return),
        "esc" | "escape" => Some(Key::Escape),
        "tab" => Some(Key::Tab),
        "space" => Some(Key::Space),
        "backspace" => Some(Key::Backspace),
        "delete" | "del" => Some(Key::Delete),
        "insert" | "ins" => Some(Key::Insert),
        "home" => Some(Key::Home),
        "end" => Some(Key::End),
        "pageup" | "pgup" => Some(Key::PageUp),
        "pagedown" | "pgdn" => Some(Key::PageDown),
        "up" => Some(Key::UpArrow),
        "down" => Some(Key::DownArrow),
        "left" => Some(Key::LeftArrow),
        "right" => Some(Key::RightArrow),
        "win" | "super" | "meta" | "lwin" | "windows" => Some(Key::Meta),
        "rwin" => Some(Key::Meta),
        "capslock" => Some(Key::CapsLock),
        "numlock" => Some(Key::Numlock),
        #[cfg(target_os = "windows")]
        "scrolllock" => Some(Key::Scroll),
        #[cfg(not(target_os = "windows"))]
        "scrolllock" => Some(Key::ScrollLock),
        "printscreen" | "prtsc" => Some(Key::PrintScr),
        "pause" => Some(Key::Pause),
        "volumemute" => Some(Key::VolumeMute),
        "volumeup" => Some(Key::VolumeUp),
        "volumedown" => Some(Key::VolumeDown),
        "playpause" | "medianext" | "mediaplaypause" => Some(Key::MediaPlayPause),
        "prevtrack" | "medianprevious" => Some(Key::MediaPrevTrack),
        "nexttrack" | "medianexttrack" => Some(Key::MediaNextTrack),
        "mediastop" => Some(Key::MediaStop),
        "f1" => Some(Key::F1),
        "f2" => Some(Key::F2),
        "f3" => Some(Key::F3),
        "f4" => Some(Key::F4),
        "f5" => Some(Key::F5),
        "f6" => Some(Key::F6),
        "f7" => Some(Key::F7),
        "f8" => Some(Key::F8),
        "f9" => Some(Key::F9),
        "f10" => Some(Key::F10),
        "f11" => Some(Key::F11),
        "f12" => Some(Key::F12),
        _ => None,
    }
}

pub(crate) fn press_key(key: &str) {
    match enigo_agent() {
        Ok(mut enigo) => {
            if let Some(mapped) = map_key(key) {
                if let Err(e) = enigo.key(mapped, Direction::Click) {
                    log().error(&format!("press_key({key:?}) failed: {e}"));
                }
            } else if key.chars().count() == 1 {
                // Single characters type directly (pyautogui.press parity).
                if let Err(e) = enigo.text(key) {
                    log().error(&format!("press_key({key:?}) failed: {e}"));
                }
            } else {
                log().warning(&format!("press_key({key:?}): unknown key name"));
            }
        }
        Err(e) => log().error(&format!("press_key backend unavailable: {e}")),
    }
}

pub(crate) fn hotkey(keys: &[&str]) {
    match enigo_agent() {
        Ok(mut enigo) => {
            let mut mapped: Vec<Key> = Vec::new();
            for key in keys {
                match map_key(key) {
                    Some(k) => mapped.push(k),
                    None if key.chars().count() == 1 => {
                        if let Some(ch) = key.chars().next() {
                            mapped.push(Key::Unicode(ch));
                        }
                    }
                    None => {
                        log().warning(&format!("hotkey({keys:?}): unknown key {key:?}"));
                        return;
                    }
                }
            }
            // pyautogui.hotkey: press all in order, release in reverse.
            for key in &mapped {
                if enigo.key(*key, Direction::Press).is_err() {
                    break;
                }
            }
            for key in mapped.iter().rev() {
                let _ = enigo.key(*key, Direction::Release);
            }
        }
        Err(e) => log().error(&format!("hotkey backend unavailable: {e}")),
    }
}

pub(crate) fn type_text(text: &str) {
    match enigo_agent() {
        Ok(mut enigo) => {
            if let Err(e) = enigo.text(text) {
                log().error(&format!("type_text failed: {e}"));
            }
        }
        Err(e) => log().error(&format!("type_text backend unavailable: {e}")),
    }
}

pub(crate) fn clipboard_copy(text: &str) {
    match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(text.to_string())) {
        Ok(()) => log().debug("clipboard_copy: done"),
        Err(e) => log().error(&format!("clipboard_copy failed: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_common_key_names() {
        assert!(matches!(map_key("ctrl"), Some(Key::LControl)));
        assert!(matches!(map_key("ENTER"), Some(Key::Return)));
        assert!(matches!(map_key("volumemute"), Some(Key::VolumeMute)));
        assert!(matches!(map_key("playpause"), Some(Key::MediaPlayPause)));
        assert!(matches!(map_key("win"), Some(Key::Meta)));
        assert!(matches!(map_key("f12"), Some(Key::F12)));
        assert_eq!(map_key("not-a-key-zzz"), None);
        // Single chars map on Windows (Key variants), type as text elsewhere.
        #[cfg(windows)]
        assert!(map_key("h").is_some());
        #[cfg(not(windows))]
        assert_eq!(map_key("h"), None);
    }
}
