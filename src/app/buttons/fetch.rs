//! HTTP fetch button action (`/fetch`) — call other apps' webhooks and
//! REST APIs from a button.
//!
//! Two message forms (the dispatch passes the raw, `<|§|>`-joined text,
//! like plugin args — the form drops empty values, so the normalized
//! space-joined text is unparseable):
//! - Form-built: `/fetch method:<|§|>POST<|§|>url:<|§|>https://…`
//!   (hidden `text` markers per field, the same convention as `/exec`'s
//!   `type:` markers; empty values arrive as bare markers).
//! - Manual: `/fetch https://host/hook` (bare URL, GET, defaults).
//!
//! Never logs header or body values (they routinely carry secrets); the
//! response body is capped at 1 MiB in memory and truncated in output.

use std::collections::HashMap;
use std::io::Read;
use std::time::Duration;

use serde_json::{json, Value};

use crate::app::utils::logger::log;

const DEFAULT_TIMEOUT_SECS: u64 = 10;
const MAX_TIMEOUT_SECS: u64 = 120;
/// Response bytes read at most (a button action must not OOM on a file URL).
const MAX_BODY_BYTES: u64 = 1024 * 1024;
/// Body characters echoed back in the result / log.
const MAX_BODY_CHARS: usize = 2000;

const METHODS: [&str; 7] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
const MARKERS: [&str; 5] = ["method:", "url:", "headers:", "body:", "timeout:"];

/// Parsed `/fetch` invocation.
#[derive(Debug, PartialEq)]
pub struct FetchRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub timeout_secs: u64,
}

/// Split the raw message into marker → value pairs. A value is every token
/// up to the next marker, re-joined — so bodies containing the `<|§|>`
/// separator survive, and empty form fields arrive as bare markers.
fn marker_values(rest: &str) -> HashMap<String, String> {
    let tokens: Vec<&str> = rest.split("<|§|>").collect();
    let positions: Vec<(usize, &str)> = tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| MARKERS.contains(token))
        .map(|(index, token)| (index, *token))
        .collect();
    let mut values = HashMap::new();
    for (i, (pos, marker)) in positions.iter().enumerate() {
        let end = positions
            .get(i + 1)
            .map(|(p, _)| *p)
            .unwrap_or(tokens.len());
        values.insert(marker.to_string(), tokens[pos + 1..end].join("<|§|>"));
    }
    values
}

/// Parse one `headers:` value (`Name: value` per line; blank lines skipped).
fn parse_headers(raw: &str) -> Result<Vec<(String, String)>, String> {
    let mut headers = Vec::new();
    for (n, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (name, value) = line
            .split_once(':')
            .map(|(n, v)| (n.trim(), v.trim()))
            .unwrap_or(("", ""));
        if name.is_empty() || value.is_empty() {
            return Err(format!(
                "fetch: invalid header on line {} (expected 'Name: value')",
                n + 1
            ));
        }
        headers.push((name.to_string(), value.to_string()));
    }
    Ok(headers)
}

fn check_url(url: &str) -> Result<String, String> {
    let url = url.trim().to_string();
    if url.is_empty() {
        return Err("fetch: missing URL".to_string());
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("fetch: URL must start with http:// or https://".to_string());
    }
    Ok(url)
}

/// Parse a `/fetch` message (raw, `<|§|>`-joined) into a request.
pub fn parse_fetch_args(message: &str) -> Result<FetchRequest, String> {
    let rest = message.strip_prefix("/fetch").unwrap_or(message).trim();
    if !rest.contains("<|§|>") && !MARKERS.iter().any(|m| rest.contains(m)) {
        // Manual form: the whole remainder is the URL.
        return Ok(FetchRequest {
            method: "GET".to_string(),
            url: check_url(rest)?,
            headers: Vec::new(),
            body: String::new(),
            timeout_secs: DEFAULT_TIMEOUT_SECS,
        });
    }
    let values = marker_values(rest);
    let method = values
        .get("method:")
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "GET".to_string());
    if !METHODS.contains(&method.as_str()) {
        return Err(format!(
            "fetch: unsupported method '{method}' ({}).",
            METHODS.join(", ")
        ));
    }
    let timeout_secs = match values.get("timeout:").map(|s| s.trim()) {
        None | Some("") => DEFAULT_TIMEOUT_SECS,
        Some(raw) => raw
            .parse::<u64>()
            .ok()
            .filter(|t| (1..=MAX_TIMEOUT_SECS).contains(t))
            .ok_or_else(|| format!("fetch: timeout must be 1–{MAX_TIMEOUT_SECS} seconds"))?,
    };
    Ok(FetchRequest {
        method,
        url: check_url(values.get("url:").map(String::as_str).unwrap_or(""))?,
        headers: parse_headers(values.get("headers:").map(String::as_str).unwrap_or(""))?,
        body: values.get("body:").cloned().unwrap_or_default(),
        timeout_secs,
    })
}

fn truncate_chars(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let truncated: String = text.chars().take(max).collect();
    format!("{truncated}…")
}

/// Port-style entry point: run the request, return the result JSON.
/// Transport errors and non-2xx statuses are failures (with the status
/// echoed back); the body is returned truncated for debugging.
pub fn fetch(message: &str) -> Value {
    let request = match parse_fetch_args(message) {
        Ok(request) => request,
        Err(message) => return json!({"success": false, "message": message}),
    };
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(request.timeout_secs))
        .build()
    {
        Ok(client) => client,
        Err(e) => return json!({"success": false, "message": format!("fetch: {e}")}),
    };
    let method = match reqwest::Method::from_bytes(request.method.as_bytes()) {
        Ok(method) => method,
        Err(_) => {
            return json!({"success": false, "message": format!("fetch: bad method '{}'", request.method)})
        }
    };
    let mut builder = client.request(method, &request.url);
    for (name, value) in &request.headers {
        let name = match reqwest::header::HeaderName::from_bytes(name.as_bytes()) {
            Ok(name) => name,
            Err(_) => {
                return json!({"success": false, "message": format!("fetch: bad header name '{name}'")})
            }
        };
        let value = match reqwest::header::HeaderValue::from_str(value) {
            Ok(value) => value,
            Err(_) => {
                return json!({"success": false, "message": format!("fetch: bad header value for '{name}'")})
            }
        };
        builder = builder.header(name, value);
    }
    if !request.body.is_empty() {
        builder = builder.body(request.body.clone());
    }
    let response = match builder.send() {
        Ok(response) => response,
        Err(e) => {
            log().error(&format!("fetch {} {}: {e}", request.method, request.url));
            return json!({"success": false, "message": format!("fetch: {e}")});
        }
    };
    let status = response.status();
    let mut capped = Vec::new();
    let body = response
        .take(MAX_BODY_BYTES)
        .read_to_end(&mut capped)
        .ok()
        .map(|_| String::from_utf8_lossy(&capped).into_owned())
        .unwrap_or_default();
    let shown = truncate_chars(&body, MAX_BODY_CHARS);
    if status.is_success() {
        log().success(&format!(
            "fetch {} {} -> {status}",
            request.method, request.url
        ));
        log().debug(&format!("fetch response body: {shown}"));
        json!({"success": true, "status": status.as_u16(), "body": shown})
    } else {
        log().error(&format!(
            "fetch {} {} -> {status}: {shown}",
            request.method, request.url
        ));
        json!({
            "success": false,
            "status": status.as_u16(),
            "message": format!("fetch: HTTP {status}"),
            "body": shown,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORM: &str = "/fetch method:<|§|>POST<|§|>url:<|§|>https://hooks.example/x<|§|>headers:<|§|>X-Token: abc<|§|>body:<|§|>{\"a\":1}<|§|>timeout:<|§|>30";

    #[test]
    fn parses_form_built_message() {
        let request = parse_fetch_args(FORM).expect("form message parses");
        assert_eq!(
            request,
            FetchRequest {
                method: "POST".to_string(),
                url: "https://hooks.example/x".to_string(),
                headers: vec![("X-Token".to_string(), "abc".to_string())],
                body: "{\"a\":1}".to_string(),
                timeout_secs: 30,
            }
        );
    }

    #[test]
    fn empty_fields_arrive_as_bare_markers() {
        // The form drops empty values, so positions shift — markers (not
        // positions) delimit fields.
        let request = parse_fetch_args(
            "/fetch method:<|§|>GET<|§|>url:<|§|>https://example.com<|§|>headers:<|§|>body:<|§|>timeout:<|§|>",
        )
        .expect("sparse form parses");
        assert_eq!(request.method, "GET");
        assert_eq!(request.url, "https://example.com");
        assert!(request.headers.is_empty());
        assert!(request.body.is_empty());
        assert_eq!(request.timeout_secs, DEFAULT_TIMEOUT_SECS);
    }

    #[test]
    fn separator_inside_body_survives() {
        let request = parse_fetch_args(
            "/fetch method:<|§|>POST<|§|>url:<|§|>https://example.com<|§|>headers:<|§|>body:<|§|>a<|§|>b<|§|>timeout:<|§|>10",
        )
        .expect("body keeps separator");
        assert_eq!(request.body, "a<|§|>b");
    }

    #[test]
    fn manual_form_is_bare_url_with_defaults() {
        let request = parse_fetch_args("/fetch https://example.com/hook").expect("manual parses");
        assert_eq!(request.method, "GET");
        assert_eq!(request.url, "https://example.com/hook");
        assert_eq!(request.timeout_secs, DEFAULT_TIMEOUT_SECS);
    }

    #[test]
    fn rejects_bad_input_before_any_network() {
        for bad in [
            "/fetch",
            "/fetch   ",
            "/fetch notaurl",
            "/fetch ftp://example.com/x",
            "/fetch method:<|§|>NOPE<|§|>url:<|§|>https://example.com",
            "/fetch method:<|§|>GET<|§|>url:<|§|>https://example.com<|§|>headers:<|§|>body:<|§|>timeout:<|§|>0",
            "/fetch method:<|§|>GET<|§|>url:<|§|>https://example.com<|§|>headers:<|§|>body:<|§|>timeout:<|§|>999",
            "/fetch method:<|§|>GET<|§|>url:<|§|>https://example.com<|§|>headers:<|§|>no-colon-here<|§|>body:<|§|>timeout:<|§|>10",
        ] {
            let failed = fetch(bad);
            assert_eq!(failed.get("success"), Some(&serde_json::json!(false)), "{bad}");
            assert!(failed.get("message").is_some(), "{bad}");
        }
    }

    #[test]
    fn missing_url_fails() {
        let failed =
            fetch("/fetch method:<|§|>GET<|§|>url:<|§|>headers:<|§|>body:<|§|>timeout:<|§|>10");
        assert_eq!(failed.get("success"), Some(&serde_json::json!(false)));
    }

    #[test]
    fn dispatch_routes_fetch_through_handle_command() {
        // Pre-network failure: proves the dispatcher branch passes the raw
        // message through without touching the network.
        let failed = crate::app::buttons::commands::handle_command("/fetch");
        assert_eq!(failed.get("success"), Some(&serde_json::json!(false)));
        assert_eq!(
            failed.get("message"),
            Some(&serde_json::json!("fetch: missing URL"))
        );
    }
}
