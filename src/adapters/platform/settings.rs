//! Native/integration changes run after the configuration lock is released.
use serde_json::Value;
pub fn apply_changes(old: &Value, new: &Value) {
    if std::env::var("WEBDECK_FAKE_EFFECTS").as_deref() == Ok("1") && cfg!(debug_assertions) {
        return;
    }
    if old.pointer("/settings/obs") != new.pointer("/settings/obs") {
        crate::adapters::integrations::obs::reload_obs();
    }
    if old.pointer("/settings/language") != new.pointer("/settings/language") {
        if let Some(lang) = new.pointer("/settings/language").and_then(Value::as_str) {
            crate::app::utils::languages::set_default_language(lang);
            crate::app::tray::change_tray_language(lang);
        }
    }
    if old.pointer("/settings/soundboard") != new.pointer("/settings/soundboard") {
        if new
            .pointer("/settings/soundboard/enabled")
            .and_then(Value::as_bool)
            == Some(true)
        {
            crate::adapters::platform::soundboard::mic::restart();
        } else {
            crate::adapters::platform::soundboard::mic::stop();
        }
    }
    if !cfg!(debug_assertions)
        && old.pointer("/settings/windows_startup") != new.pointer("/settings/windows_startup")
    {
        #[cfg(any(windows, target_os = "linux"))]
        if new
            .pointer("/settings/windows_startup")
            .and_then(Value::as_bool)
            == Some(true)
        {
            crate::app::on_start::utils::create_startup_shortcut();
        } else {
            #[cfg(target_os = "linux")]
            crate::app::on_start::utils::remove_startup_shortcut();
            #[cfg(windows)]
            if let Ok(appdata) = std::env::var("APPDATA") {
                let _ = std::fs::remove_file(format!(
                    "{appdata}\\Microsoft\\Windows\\Start Menu\\Programs\\Startup\\WebDeck.lnk"
                ));
            }
        }
    }
}
