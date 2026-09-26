//! Debug console — port of `console.py`.
//!
//! REPL that POSTs typed messages to `/send-data`, printing the round-trip
//! time. Run with `cargo run --bin console`.

#![allow(dead_code)]

use std::io::{self, BufRead, Write};
use std::time::Instant;

use webdeck::app::utils::{get_local_ip, logger::log, settings::get_config, working_dir};

#[tokio::main]
async fn main() {
    working_dir::chdir_base();

    let config = get_config::get_config(false, false);
    let port = config
        .get("url")
        .and_then(|u| u.get("port"))
        .and_then(|p| p.as_u64())
        .unwrap_or(5000);
    let host = get_local_ip::get_local_ip().unwrap_or_else(|_| "127.0.0.1".to_string());
    let url = format!("http://{host}:{port}/send-data");

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
        let result = client
            .post(&url)
            .json(&serde_json::json!({ "message": message }))
            .send()
            .await;
        let elapsed = start.elapsed().as_secs_f64();

        match result {
            Ok(response) if response.status().is_success() => {
                println!("success! {elapsed:.2}s");
            }
            Ok(response) => println!("Error: {}", response.status()),
            Err(e) => println!("Error: {e}"),
        }
    }
}
