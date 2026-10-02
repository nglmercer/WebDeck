use crate::domain::{Error, Result};
use std::sync::Arc;
pub fn run(url: String, shutdown: Arc<tokio::sync::Notify>) -> Result<()> {
    use tao::{
        event::{Event, StartCause},
        event_loop::{ControlFlow, EventLoopBuilder},
    };
    use tray_icon::{
        menu::{Menu, MenuEvent, MenuItem},
        Icon, TrayIconBuilder,
    };
    let mut builder = EventLoopBuilder::new();
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
    let events = builder.build();
    let menu = Menu::new();
    let deck = MenuItem::new("Open WebDeck", true, None);
    let qr = MenuItem::new("Show QR code", true, None);
    let quit = MenuItem::new("Quit", true, None);
    menu.append_items(&[&deck, &qr, &quit])
        .map_err(|_| Error::execution())?;
    let icon = Icon::from_rgba([102, 84, 232, 255].repeat(32 * 32), 32, 32)
        .map_err(|_| Error::execution())?;
    let mut tray = None;
    events.run(move |event, _, flow| {
        *flow = ControlFlow::WaitUntil(
            std::time::Instant::now() + std::time::Duration::from_millis(100),
        );
        if let Event::NewEvents(StartCause::Init) = event {
            tray = TrayIconBuilder::new()
                .with_tooltip("WebDeck v2")
                .with_menu(Box::new(menu.clone()))
                .with_icon(icon.clone())
                .build()
                .ok();
        }
        if let Ok(e) = MenuEvent::receiver().try_recv() {
            if e.id == quit.id() {
                shutdown.notify_one();
                *flow = ControlFlow::Exit;
            } else if e.id == deck.id() {
                let _ = crate::native::open(&url);
            } else if e.id == qr.id() {
                if let Ok(exe) = std::env::current_exe() {
                    let child = exe.with_file_name(if cfg!(windows) {
                        "webdeck-qr.exe"
                    } else {
                        "webdeck-qr"
                    });
                    let _ = std::process::Command::new(child)
                        .args(["--text", &url])
                        .spawn();
                }
            }
        }
        let _ = &tray;
    })
}
