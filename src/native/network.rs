use super::*;

pub(super) fn fetch(
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
    let mut b = Vec::new();
    r.take(65537)
        .read_to_end(&mut b)
        .map_err(|_| Error::execution())?;
    if !status.is_success() {
        return Err(Error::execution());
    }
    let truncated = b.len() > 65536;
    b.truncate(65536);
    Ok(json!({"status":status.as_u16(),"body":String::from_utf8_lossy(&b),"truncated":truncated}))
}
