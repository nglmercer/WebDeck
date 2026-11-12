use crate::domain::{Error, Result};
use image::imageops::FilterType;
use std::sync::Arc;
use tray_icon::Icon;
use tray_icon::{menu::MenuEvent, TrayIconEvent};

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
        menu::{Menu, MenuItem},
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
    let qr = MenuItem::new(&labels[2], true, None);
    let quit = MenuItem::new(&labels[3], true, None);
    menu.append_items(&[&deck, &settings, &qr, &quit])
        .map_err(|_| Error::execution())?;
    let icon = load_tray_icon()?;
    let mut tray = None;
    events.run_return(move |event, _, flow| {
        let selected = config.language();
        if selected != language {
            if let Ok(labels) = menu_labels(&selected) {
                for (item, label) in [&deck, &settings, &qr, &quit].iter().zip(labels) {
                    item.set_text(label);
                }
                language = selected;
            }
        }
        *flow = ControlFlow::Wait;
        match event {
            Event::NewEvents(StartCause::Init) => {
                match TrayIconBuilder::new()
                    .with_tooltip("WebDeck")
                    .with_menu(Box::new(menu.clone()))
                    .with_icon(icon.clone())
                    .build()
                {
                    Ok(icon) => tray = Some(icon),
                    Err(e) => {
                        eprintln!("Tray: cannot create icon: {e}");
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
                        eprintln!("QR: {e}");
                    }
                });
            }
            Event::UserEvent(TrayEvent::Menu(e)) if e.id == settings.id() => {
                if let Err(e) = crate::capabilities::open(&format!("{url}#settings")) {
                    eprintln!("Tray: {e}");
                }
            }
            Event::UserEvent(TrayEvent::Menu(e)) if e.id == deck.id() => {
                if let Err(e) = crate::capabilities::open(&url) {
                    eprintln!("Tray: {e}");
                }
            }
            Event::UserEvent(TrayEvent::Icon(TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            })) => {
                if let Err(e) = crate::capabilities::open(&url) {
                    eprintln!("Tray: {e}");
                }
            }
            _ => {}
        }
        let _ = &tray;
    });
    MenuEvent::set_event_handler(None::<fn(MenuEvent)>);
    TrayIconEvent::set_event_handler(None::<fn(TrayIconEvent)>);
    Ok(())
}

fn menu_labels(language: &str) -> Result<[String; 4]> {
    let catalog = crate::server::translations::dictionary(language)?;
    let mut labels = Vec::new();
    for key in [
        "common.openWebDeck",
        "common.settings",
        "connections.showQr",
        "commands.exit",
    ] {
        labels.push(catalog.get(key).cloned().ok_or_else(Error::execution)?);
    }
    labels.try_into().map_err(|_| Error::execution())
}

#[cfg(test)]
mod tests {
    use super::*;
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
            assert_eq!(labels[2], dictionary["connections.showQr"]);
            assert_eq!(labels[3], dictionary["commands.exit"]);
        }
        assert_eq!(menu_labels("zz_ZZ").unwrap(), menu_labels("en_US").unwrap());
    }
}
