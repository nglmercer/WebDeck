//! Binary entry point — port of `run.py`.
//!
//! Startup sequence (same as Python):
//! 1. `chdir_base()` so relative paths (`webdeck/`, `.config/`, …) resolve.
//! 2. `parse_args()` + load config (with update check + save).
//! 3. Windows UAC self-elevation when `settings.app_admin`.
//! 4. Single-instance guard unless `--force-start`.
//! 5. Init translations, spawn server task + welcome popup.
//! 6. Run tray icon (blocking) or wait for Ctrl+C with `--no-tray`.

#![allow(dead_code)]

use webdeck::app::utils::{
    args, is_opened, languages, logger::log, settings::get_config, show_error, welcome_popup,
    working_dir,
};

#[tokio::main]
async fn main() {
    working_dir::chdir_base();
    args::parse_args();

    let config = get_config::get_config(true, true);
    let default_lang = config
        .get("settings")
        .and_then(|s| s.get("language"))
        .and_then(|v| v.as_str())
        .unwrap_or("en_US")
        .to_string();

    #[cfg(windows)]
    {
        use windows::core::{w, HSTRING};
        use windows::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteW};
        use windows::Win32::UI::WindowsAndMessaging::SW_NORMAL;

        let wants_admin = config
            .get("settings")
            .and_then(|s| s.get("app_admin"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        // Port of the `ctypes.windll.shell32.IsUserAnAdmin` / `ShellExecuteW`
        // runas dance in `run.py`.
        let is_admin = unsafe { IsUserAnAdmin().as_bool() };
        if wants_admin && !is_admin && !args::get_args().no_admin {
            let exe = std::env::current_exe()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let params = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
            unsafe {
                let _ = ShellExecuteW(
                    None,
                    w!("runas"),
                    &HSTRING::from(exe.as_str()),
                    &HSTRING::from(params.as_str()),
                    None,
                    SW_NORMAL,
                );
            }
            std::process::exit(0);
        }
    }
    #[cfg(not(windows))]
    {
        let _ = &config;
    }

    if !is_opened::is_opened() || args::get_args().force_start {
        webdeck::application::lifecycle::activate();
        log().info("Starting WebDeck");

        log().info("Loading translations");
        languages::init(
            "webdeck/translations",
            Some("webdeck/translations/misc"),
            &default_lang,
        );

        log().info("Starting server task");
        let server_handle = tokio::spawn(async {
            if let Err(e) = webdeck::app::server::run_server().await {
                webdeck::application::lifecycle::request_shutdown();
                // Native error dialogs block: run off the async worker.
                tokio::task::block_in_place(|| {
                    show_error::show_error(
                        Some("Server task failed"),
                        "WebDeck Error",
                        true,
                        Some(&e as &dyn std::fmt::Debug),
                    );
                });
            }
        });

        let popup_handle = tokio::task::spawn_blocking(welcome_popup::show_popup);

        if !args::get_args().no_tray {
            log().info("Initializing tray icon");
            // `create_tray_icon()` blocks like `pystray.Icon.run()`; run it on a
            // blocking thread so the server task keeps running.
            let tray_result =
                tokio::task::spawn_blocking(webdeck::app::tray::create_tray_icon).await;
            match tray_result {
                Ok(()) => {}
                Err(e) => {
                    #[cfg(windows)]
                    show_error::show_error(
                        None,
                        "WebDeck Error",
                        true,
                        Some(&e as &dyn std::fmt::Debug),
                    );
                    #[cfg(not(windows))]
                    log().exception(
                        &e,
                        Some("Failed to initialize tray icon"),
                        false,
                        true,
                        true,
                    );
                }
            }
        } else {
            log().info("Running without tray icon");
            tokio::select! {
                _ = tokio::signal::ctrl_c() => webdeck::application::lifecycle::request_shutdown(),
                _ = webdeck::application::lifecycle::shutdown_requested() => (),
            }
        }

        webdeck::application::lifecycle::request_shutdown();
        let _ = server_handle.await;
        let _ = popup_handle.await;
        if webdeck::application::lifecycle::restarting() {
            if let Ok(exe) = std::env::current_exe() {
                let _ = std::process::Command::new(exe)
                    .args(std::env::args().skip(1))
                    .arg("--force-start")
                    .spawn();
            }
        }
    }
}
