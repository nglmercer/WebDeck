//! Debug console — port of `console.py`.
//!
//! REPL that POSTs command requests to `/api/v2/commands`, printing the round-trip
//! time. Run with `cargo run --bin console`.

#![allow(dead_code)]

use std::io::{self, BufRead, Write};
use std::time::Instant;

use webdeck::app::utils::{logger::log, settings::get_config, working_dir};

#[tokio::main]
async fn main() {
    working_dir::chdir_base();

    let config = get_config::get_config(false, false);
    let port = config
        .get("url")
        .and_then(|u| u.get("port"))
        .and_then(|p| p.as_u64())
        .unwrap_or(5000);
    let host = std::env::var("WEBDECK_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let token = std::env::var("WEBDECK_DEVICE_TOKEN").ok();
    let url = format!("http://{host}:{port}/api/v2/commands");

    let client = reqwest::Client::new();
    let stdin = io::stdin();

    loop {
        print!("Message: ");
        let _ = io::stdout().flush();
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                log().exception(&e, Some("Failed to read stdin"), true, false, true);
                break;
            }
        }
        let message = line.trim_end_matches(['\r', '\n']).to_string();

        let start = Instant::now();
        let mut request = client
            .post(&url)
            .json(&serde_json::json!({"message": message}));
        if let Some(token) = &token {
            request = request.bearer_auth(token);
        }
        let result = request.send().await;
        let elapsed = start.elapsed().as_secs_f64();

        match result {
            Ok(response) => {
                let status = response.status();
                match response.json::<serde_json::Value>().await {
                    Ok(result) if status.is_success() && result["state"] == "completed" => {
                        println!("completed {elapsed:.2}s")
                    }
                    Ok(result) => println!(
                        "Command failed ({status}): {}",
                        result["code"].as_str().unwrap_or("invalid_response")
                    ),
                    Err(_) => println!("Invalid server response ({status}); outcome unknown"),
                }
            }
            Err(_) => println!("Transport failed; outcome unknown. No retry was attempted."),
        }
    }
}
