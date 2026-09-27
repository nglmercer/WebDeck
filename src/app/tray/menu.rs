//! Tray menu construction + dispatch (extracted from `tray.rs`).

use tray_icon::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};

use super::state::update_language;
use super::ServerState;
use super::windows::{change_port_prompt, open_config, show_qrcode};
use crate::app::buttons::system::openfile::openfile;
use crate::app::utils::exit::exit_program;
use crate::app::utils::firewall::fix_firewall_permission;
use crate::app::utils::languages::{get_language, get_languages_info, text};
use crate::app::utils::logger::log;
use crate::app::utils::restart::restart_program;

/// Port of `generate_menu`. Item ids are stable dispatch keys.
pub(crate) fn generate_menu(language: &str, server_status: ServerState) -> Menu {
    log().info(&format!("Server status updated: {}", server_status as u8));

    let lang = Some(language);
    let status_text = match server_status {
        ServerState::Loading => text(Some("server_loading"), lang),
        ServerState::Running => text(Some("server_online"), lang),
        ServerState::Stopped => text(Some("server_offline"), lang),
    };

    let menu = Menu::new();
    let item_qr = MenuItem::with_id("qr", text(Some("qr_code"), lang), true, None);

    let submenu = Submenu::new(text(Some("options"), lang), true);
    let item_open_config =
        MenuItem::with_id("open_config", text(Some("open_config"), lang), true, None);

    // Language submenu: `native_name (code)` unless identical, misc
    // languages after a separator, current language checked.
    let lang_menu = Submenu::new(text(Some("language"), lang), true);
    let resolved = get_language(Some(language));
    let mut infos = get_languages_info();
    infos.sort_by(|a, b| a.misc.cmp(&b.misc));
    let mut with_separator = false;
    for info in &infos {
        if info.misc && !with_separator {
            let _ = lang_menu.append(&PredefinedMenuItem::separator());
            with_separator = true;
        }
        let label = if info.native_name != info.code {
            format!("{} ({})", info.native_name, info.code)
        } else {
            info.code.clone()
        };
        let check = CheckMenuItem::with_id(
            format!("lang:{}", info.code),
            label,
            true,
            info.code == resolved,
            None,
        );
        let _ = lang_menu.append(&check);
    }

    let item_restart = MenuItem::with_id(
        "restart",
        text(Some("restart_application"), lang),
        true,
        None,
    );
    let item_edit_port = MenuItem::with_id("edit_port", text(Some("edit_port"), lang), true, None);
    let item_fix_firewall =
        MenuItem::with_id("fix_firewall", text(Some("fix_firewall"), lang), true, None);
    let _ = submenu.append_items(&[
        &item_open_config,
        &lang_menu,
        &item_restart,
        &item_edit_port,
        &item_fix_firewall,
    ]);

    let item_server = MenuItem::with_id(
        "server_status",
        format!("{} {status_text}", text(Some("server_status"), lang)),
        true,
        None,
    );
    let item_issue =
        MenuItem::with_id("report_issue", text(Some("report_issue"), lang), true, None);
    let item_exit = MenuItem::with_id("exit", text(Some("exit"), lang), true, None);

    let _ = menu.append_items(&[&item_qr, &submenu, &item_server]);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append_items(&[&item_issue, &item_exit]);
    menu
}

/// Dispatch a tray menu activation by item id.
pub(crate) fn dispatch_menu(id: &str) {
    match id {
        "qr" => show_qrcode(),
        "open_config" | "server_status" => open_config(),
        "restart" => restart_program(),
        "edit_port" => change_port_prompt(),
        "fix_firewall" => fix_firewall_permission(),
        "report_issue" => {
            openfile("https://github.com/Lenochxd/WebDeck/issues");
        }
        "exit" => exit_program(true, false),
        lang if lang.starts_with("lang:") => {
            update_language(lang.trim_start_matches("lang:"));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_builds_with_all_states() {
        crate::app::utils::languages::init(
            "webdeck/translations",
            Some("webdeck/translations/misc"),
            "en_US",
        );
        for state in [
            ServerState::Running,
            ServerState::Stopped,
            ServerState::Loading,
        ] {
            let _ = generate_menu("en", state);
        }
    }
}
