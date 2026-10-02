use std::io::{self, BufRead};
use webdeck::{contracts::CommandRequest, domain};
#[tokio::main]
async fn main() {
    let host = std::env::var("WEBDECK_URL").unwrap_or_else(|_| "http://127.0.0.1:5000".into());
    let token = std::env::var("WEBDECK_DEVICE_TOKEN").ok();
    let c = reqwest::Client::new();
    println!("Enter one typed JSON command per line.");
    for line in io::stdin().lock().lines() {
        let Ok(line) = line else {
            break;
        };
        let command = match serde_json::from_str(&line) {
            Ok(c) => c,
            Err(_) => {
                eprintln!("Invalid JSON command");
                continue;
            }
        };
        let request = CommandRequest {
            request_id: domain::id(),
            command,
        };
        if domain::validate_command(&request.command).is_err() {
            eprintln!("Invalid v2 command");
            continue;
        }
        let mut r = c.post(format!("{host}/api/v2/commands")).json(&request);
        if let Some(t) = &token {
            r = r.bearer_auth(t);
        }
        match r.send().await {
            Ok(r) => println!("{}", r.text().await.unwrap_or_default()),
            Err(_) => {
                eprintln!("Request failed; execution outcome may be unknown. No retry was sent.")
            }
        }
    }
}
