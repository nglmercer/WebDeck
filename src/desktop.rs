use crate::domain::{Error, Result};
use std::sync::Arc;
use tray_icon::{menu::MenuEvent, TrayIconEvent};
#[derive(Debug)]
enum TrayEvent {
    Menu(MenuEvent),
    Icon(TrayIconEvent),
}
pub fn run(url: String, shutdown: Arc<tokio::sync::Notify>) -> Result<()> {
    use tao::{
        event::{Event, StartCause},
        event_loop::{ControlFlow, EventLoopBuilder},
    };
    use tray_icon::{
        menu::{Menu, MenuItem},
        Icon, MouseButton, MouseButtonState, TrayIconBuilder,
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
    let deck = MenuItem::new("Open WebDeck", true, None);
    let settings = MenuItem::new("Settings", true, None);
    let qr = MenuItem::new("Show QR code", true, None);
    let quit = MenuItem::new("Quit", true, None);
    menu.append_items(&[&deck, &settings, &qr, &quit])
        .map_err(|_| Error::execution())?;
    let icon = Icon::from_rgba([102, 84, 232, 255].repeat(32 * 32), 32, 32)
        .map_err(|_| Error::execution())?;
    let mut tray = None;
    events.run_return(move |event, _, flow| {
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
                let text = url.clone();
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
