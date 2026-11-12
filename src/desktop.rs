use crate::domain::{Error, Result};
use image::imageops::FilterType;
use std::sync::Arc;
use tray_icon::Icon;
use tray_icon::{menu::MenuEvent, TrayIconEvent};

/// Visible startup errors still reach Explorer-launched GUI applications.
pub fn report_error(message: &str) {
    eprintln!("WebDeck: {message}");
    #[cfg(windows)]
    let _ = rfd::MessageDialog::new()
        .set_title("WebDeck")
        .set_description(message)
        .set_level(rfd::MessageLevel::Error)
        .show();
}

#[derive(Clone, Copy)]
enum WebPage {
    Deck,
    Settings,
    Devices,
    Tutorials,
}

fn web_link(base: &str, page: WebPage) -> Result<String> {
    let mut url = reqwest::Url::parse(base).map_err(|_| Error::execution())?;
    url.set_fragment(None);
    if !matches!(page, WebPage::Deck) {
        let mut query = url.query_pairs_mut();
        query.append_pair("view", "settings");
        if let Some(section) = match page {
            WebPage::Devices => Some("devices"),
            WebPage::Tutorials => Some("tutorials"),
            _ => None,
        } {
            query.append_pair("section", section);
        }
    }
    Ok(url.into())
}

const TRAY_ICON: &[u8] = include_bytes!("../static/icons/icon.ico");

fn load_tray_icon() -> Result<Icon> {
    let image =
        image::load_from_memory_with_format(TRAY_ICON, image::ImageFormat::Ico).map_err(|err| {
            eprintln!("Tray: failed to decode icon: {err}");
            Error::execution()
        })?;

    // Tray icons are tiny. Explicitly rasterizing avoids relying on
    // whatever size happens to be stored first in the ICO.
    let rgba = image.resize_exact(32, 32, FilterType::Lanczos3).to_rgba8();

    let (width, height) = rgba.dimensions();

    Icon::from_rgba(rgba.into_raw(), width, height).map_err(|err| {
        eprintln!("Tray: failed to create icon: {err}");
        Error::execution()
    })
}
#[derive(Debug)]
enum TrayEvent {
    Menu(MenuEvent),
    Icon(TrayIconEvent),
}
pub fn run(
    url: String,
    qr_url: String,
    network: Arc<crate::network::Network>,
    runtime: tokio::runtime::Handle,
    shutdown: Arc<tokio::sync::Notify>,
    config: Arc<crate::storage::ConfigStore>,
) -> Result<()> {
    use tao::{
        event::{Event, StartCause},
        event_loop::{ControlFlow, EventLoopBuilder},
    };
    use tray_icon::{
        menu::{Menu, MenuItem, PredefinedMenuItem},
        MouseButton, MouseButtonState, TrayIconBuilder,
    };
    let mut builder = EventLoopBuilder::<TrayEvent>::with_user_event();
    #[cfg(target_os = "linux")]
    {
        use tao::platform::unix::EventLoopBuilderExtUnix;
        builder.with_any_thread(true);
    }
    #[cfg(windows)]
    {
        use tao::platform::windows::EventLoopBuilderExtWindows;
        builder.with_any_thread(true);
    }
    let mut events = builder.build();
    use tao::platform::run_return::EventLoopExtRunReturn;
    let proxy = events.create_proxy();
    MenuEvent::set_event_handler(Some(move |e| {
        let _ = proxy.send_event(TrayEvent::Menu(e));
    }));
    let proxy = events.create_proxy();
    TrayIconEvent::set_event_handler(Some(move |e| {
        let _ = proxy.send_event(TrayEvent::Icon(e));
    }));
    let menu = Menu::new();
    let mut language = config.language();
    let labels = menu_labels(&language)?;
    let deck = MenuItem::new(&labels[0], true, None);
    let settings = MenuItem::new(&labels[1], true, None);
    let devices = MenuItem::new(&labels[2], true, None);
    let tutorials = MenuItem::new(&labels[3], true, None);
    let qr = MenuItem::new(&labels[4], true, None);
    let quit = MenuItem::new(&labels[5], true, None);
    let status = MenuItem::new(&labels[6], false, None);
    let tooltip = format!("{}\n{url}", labels[6]);
    let deck_url = web_link(&url, WebPage::Deck)?;
    let settings_url = web_link(&url, WebPage::Settings)?;
    let devices_url = web_link(&url, WebPage::Devices)?;
    let tutorials_url = web_link(&url, WebPage::Tutorials)?;
    menu.append_items(&[
        &status,
        &PredefinedMenuItem::separator(),
        &deck,
        &settings,
        &devices,
        &tutorials,
        &qr,
        &PredefinedMenuItem::separator(),
        &quit,
    ])
    .map_err(|_| Error::execution())?;
    let icon = load_tray_icon()?;
    let mut tray: Option<tray_icon::TrayIcon> = None;
    let failure = std::rc::Rc::new(std::cell::Cell::new(None));
    let failed = failure.clone();
    events.run_return(move |event, _, flow| {
        let selected = config.language();
        if selected != language {
            if let Ok(labels) = menu_labels(&selected) {
                for (item, label) in [&deck, &settings, &devices, &tutorials, &qr, &quit, &status]
                    .iter()
                    .zip(&labels)
                {
                    item.set_text(label);
                }
                if let Some(tray) = &tray {
                    let _ = tray.set_tooltip(Some(format!("{}\n{url}", labels[6])));
                }
                language = selected;
            }
        }
        *flow = ControlFlow::Wait;
        match event {
            Event::NewEvents(StartCause::Init) => {
                match TrayIconBuilder::new()
                    .with_tooltip(&tooltip)
                    .with_menu_on_left_click(false)
                    .with_menu(Box::new(menu.clone()))
                    .with_icon(icon.clone())
                    .build()
                {
                    Ok(icon) => tray = Some(icon),
                    Err(e) => {
                        failed.set(Some(Error::new(
                            crate::contracts::ErrorCode::ExecutionFailed,
                            format!("Cannot create tray icon: {e}"),
                        )));
                        *flow = ControlFlow::Exit;
                    }
                }
            }
            Event::UserEvent(TrayEvent::Menu(e)) if e.id == quit.id() => {
                shutdown.notify_one();
                *flow = ControlFlow::Exit;
            }
            Event::UserEvent(TrayEvent::Menu(e)) if e.id == qr.id() => {
                let current = runtime.block_on(network.status());
                let text = if current.url.is_empty() {
                    qr_url.clone()
                } else {
                    current.url
                };
                std::thread::spawn(move || {
                    if let Err(e) = crate::qr::show(&text) {
                        report_error(&format!("QR: {e}"));
                    }
                });
            }
            Event::UserEvent(TrayEvent::Menu(e)) if e.id == devices.id() => {
                if let Err(e) = crate::capabilities::open(&devices_url) {
                    report_error(&e.to_string());
                }
            }
            Event::UserEvent(TrayEvent::Menu(e)) if e.id == tutorials.id() => {
                if let Err(e) = crate::capabilities::open(&tutorials_url) {
                    report_error(&e.to_string());
                }
            }
            Event::UserEvent(TrayEvent::Menu(e)) if e.id == settings.id() => {
                if let Err(e) = crate::capabilities::open(&settings_url) {
                    report_error(&e.to_string());
                }
            }
            Event::UserEvent(TrayEvent::Menu(e)) if e.id == deck.id() => {
                if let Err(e) = crate::capabilities::open(&deck_url) {
                    report_error(&e.to_string());
                }
            }
            Event::UserEvent(TrayEvent::Icon(TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            })) => {
                if let Err(e) = crate::capabilities::open(&deck_url) {
                    report_error(&e.to_string());
                }
            }
            _ => {}
        }
        let _ = &tray;
    });
    MenuEvent::set_event_handler(None::<fn(MenuEvent)>);
    TrayIconEvent::set_event_handler(None::<fn(TrayIconEvent)>);
    if let Some(error) = failure.take() {
        Err(error)
    } else {
        Ok(())
    }
}

fn menu_labels(language: &str) -> Result<[String; 7]> {
    let catalog = crate::server::translations::dictionary(language)?;
    let mut labels = Vec::new();
    for key in [
        "common.openWebDeck",
        "common.settings",
        "connections.devicesConnections",
        "settings.settingsHelp",
        "connections.showQr",
        "commands.exit",
        "desktop.running",
    ] {
        labels.push(catalog.get(key).cloned().ok_or_else(Error::execution)?);
    }
    labels.try_into().map_err(|_| Error::execution())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tray_links_target_settings_and_tutorials_without_changing_the_host() {
        let base = "http://127.0.0.1:5353/";
        assert_eq!(web_link(base, WebPage::Deck).unwrap(), base);
        for (page, expected) in [
            (WebPage::Settings, "?view=settings"),
            (WebPage::Devices, "?view=settings&section=devices"),
            (WebPage::Tutorials, "?view=settings&section=tutorials"),
        ] {
            assert_eq!(web_link(base, page).unwrap(), format!("{base}{expected}"));
        }
        assert_eq!(
            web_link("http://[::1]:5353/", WebPage::Tutorials).unwrap(),
            "http://[::1]:5353/?view=settings&section=tutorials"
        );
    }
    #[test]
    fn tray_labels_share_the_selected_web_dictionary_in_every_language() {
        for (locale, settings) in [
            ("en_US", "Settings"),
            ("es_ES", "Configuración"),
            ("de_DE", "Einstellungen"),
            ("fr_FR", "Paramètres"),
            ("ko_KR", "설정"),
            ("pl_PL", "Ustawienia"),
            ("ru_RU", "Настройки"),
        ] {
            let labels = menu_labels(locale).unwrap();
            assert!(labels[0].contains("WebDeck"));
            assert_eq!(labels[1], settings);
            let dictionary = crate::server::translations::dictionary(locale).unwrap();
            assert_eq!(labels[2], dictionary["connections.devicesConnections"]);
            assert_eq!(labels[3], dictionary["settings.settingsHelp"]);
            assert_eq!(labels[4], dictionary["connections.showQr"]);
            assert_eq!(labels[5], dictionary["commands.exit"]);
            assert_eq!(labels[6], dictionary["desktop.running"]);
        }
        assert_eq!(menu_labels("zz_ZZ").unwrap(), menu_labels("en_US").unwrap());
    }
}
