use super::*;

fn fetch_response(
    method: &str,
    url: &str,
    headers: &BTreeMap<String, String>,
    body: &str,
    timeout: Duration,
) -> Result<Value> {
    let c = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|_| Error::execution())?;
    let method = reqwest::Method::from_bytes(method.as_bytes()).map_err(|_| Error::invalid())?;
    let mut r = c.request(method, url).body(body.to_owned());
    for (k, v) in headers {
        r = r.header(k, v);
    }
    let r = r.send().map_err(|_| Error::execution())?;
    let status = r.status();
    let response_headers: BTreeMap<String, String> = r
        .headers()
        .iter()
        .take(128)
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.to_string(), value.to_owned()))
        })
        .collect();
    if response_headers
        .iter()
        .map(|(k, v)| k.len() + v.len())
        .sum::<usize>()
        > 16384
    {
        return Err(Error::execution());
    }
    let mut b = Vec::new();
    r.take(65537)
        .read_to_end(&mut b)
        .map_err(|_| Error::execution())?;
    let truncated = b.len() > 65536;
    b.truncate(65536);
    Ok(
        json!({"status":status.as_u16(),"body":String::from_utf8_lossy(&b),"truncated":truncated,"headers":response_headers}),
    )
}

impl Platform {
    pub(super) fn network_request(&self, input: &Value, context: &Context) -> Result<Value> {
        let operation = "network.fetch";
        context.check(crate::runtime::capabilities::required_capability(
            operation,
        )?)?;
        match operation {
            "network.fetch" => {
                let method = input["method"].as_str().ok_or_else(Error::invalid)?;
                let url = input["url"].as_str().ok_or_else(Error::invalid)?;
                let parsed = reqwest::Url::parse(url).map_err(|_| Error::invalid())?;
                if !matches!(parsed.scheme(), "http" | "https")
                    || !parsed.username().is_empty()
                    || parsed.password().is_some()
                {
                    return Err(Error::invalid());
                }
                let headers: BTreeMap<String, String> =
                    serde_json::from_value(input["headers"].clone())
                        .map_err(|_| Error::invalid())?;
                if headers.len() > 128
                    || headers
                        .iter()
                        .map(|(k, v)| k.len() + v.len())
                        .sum::<usize>()
                        > 16384
                {
                    return Err(Error::invalid());
                }
                let body = input["body"].as_str().ok_or_else(Error::invalid)?;
                if body.len() > 65536 {
                    return Err(Error::invalid());
                }
                let timeout = input["timeoutMs"]
                    .as_u64()
                    .filter(|n| *n > 0 && *n <= 30000)
                    .ok_or_else(Error::invalid)?;
                fetch_response(
                    method,
                    url,
                    &headers,
                    body,
                    context.remaining(Capability::Network, Duration::from_millis(timeout))?,
                )
            }
            _ => Err(Error::invalid()),
        }
    }
}
