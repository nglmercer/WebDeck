//! Image imports share the same durable asset references as uploads. Network access
//! occurs only during an explicit local administrator import or refresh.
use crate::{
    contracts::{ErrorCode, FileSource, ImageImport},
    domain::{Error, Result},
    storage::{atomic_replace, protected_write, Assets},
};
use std::{
    fs::{self, OpenOptions},
    io::{Cursor, Read},
    net::{IpAddr, SocketAddr, ToSocketAddrs},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
const MAX_BYTES: u64 = 16 * 1024 * 1024;
fn invalid(message: &str) -> Error {
    Error::new(ErrorCode::InvalidInput, message)
}

fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => {
            let [a, b, c, _] = v.octets();
            !(v.is_private()
                || v.is_loopback()
                || v.is_link_local()
                || v.is_broadcast()
                || v.is_documentation()
                || v.is_unspecified()
                || v.is_multicast()
                || a == 0
                || a >= 240
                || (a == 100 && (64..=127).contains(&b))
                || (a == 198 && (b == 18 || b == 19))
                || (a == 192 && b == 0 && c == 0)
                || (a == 192 && b == 88 && c == 99))
        }
        IpAddr::V6(v) => {
            // Public global unicast only; exclude documentation and transition
            // ranges, including IPv4-mapped addresses and NAT64.
            let s = v.segments();
            (s[0] & 0xe000) == 0x2000
                && s[0] != 0x2002
                && !(s[0] == 0x2001 && s[1] < 0x0200)
                && !(s[0] == 0x2001 && s[1] == 0x0db8)
                && !(s[0] == 0x3fff && s[1] <= 0x0fff)
        }
    }
}
fn parse_url(value: &str) -> Result<reqwest::Url> {
    let url = reqwest::Url::parse(value).map_err(|_| invalid("Invalid image URL"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || value.len() > 4096
    {
        return Err(invalid("Use an HTTP(S) image URL without credentials"));
    }
    Ok(url)
}
fn destination(url: &reqwest::Url) -> Result<(String, Vec<SocketAddr>)> {
    let host = url
        .host_str()
        .ok_or_else(|| invalid("Invalid image host"))?;
    let host = host.trim_start_matches('[').trim_end_matches(']');
    let addresses: Vec<_> = (host, url.port_or_known_default().unwrap_or(443))
        .to_socket_addrs()
        .map_err(|_| invalid("Cannot resolve image host"))?
        .collect();
    if addresses.is_empty() || addresses.iter().any(|a| !public_ip(a.ip())) {
        return Err(invalid(
            "Image URLs must resolve to public internet addresses",
        ));
    }
    Ok((host.to_owned(), addresses))
}
fn download(value: &str) -> Result<Vec<u8>> {
    download_using(value, destination)
}
fn download_using(
    value: &str,
    resolve: impl Fn(&reqwest::Url) -> Result<(String, Vec<SocketAddr>)>,
) -> Result<Vec<u8>> {
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut url = parse_url(value)?;
    for _ in 0..=5 {
        let (host, addresses) = resolve(&url)?;
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(|| invalid("Image download timed out"))?;
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .resolve_to_addrs(&host, &addresses)
            .timeout(remaining)
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|_| invalid("Cannot create image downloader"))?;
        let response = client
            .get(url.clone())
            .send()
            .map_err(|_| invalid("Image download failed"))?;
        if response.status().is_redirection() {
            let target = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or_else(|| invalid("Invalid image redirect"))?;
            let next = url
                .join(target)
                .map_err(|_| invalid("Invalid image redirect"))?;
            if url.scheme() == "https" && next.scheme() != "https" {
                return Err(invalid("Image redirect cannot downgrade HTTPS"));
            }
            url = parse_url(next.as_str())?;
            continue;
        }
        if !response.status().is_success() {
            return Err(invalid("Image server returned an unsuccessful response"));
        }
        if response.content_length().is_some_and(|n| n > MAX_BYTES) {
            return Err(invalid("Image exceeds the 16 MiB limit"));
        }
        return bounded_read(response);
    }
    Err(invalid("Too many image redirects"))
}
fn bounded_read(reader: impl Read) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid("Cannot read image"))?;
    if bytes.len() as u64 > MAX_BYTES || bytes.is_empty() {
        return Err(invalid("Image must be between 1 byte and 16 MiB"));
    }
    Ok(bytes)
}
fn local_bytes(value: &str) -> Result<Vec<u8>> {
    let path = Path::new(value);
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(invalid("Choose an absolute local image path"));
    }
    file_bytes(path)
}
fn file_bytes(path: &Path) -> Result<Vec<u8>> {
    if !fs::symlink_metadata(path).is_ok_and(|m| m.is_file() && m.len() <= MAX_BYTES) {
        return Err(invalid("Choose a regular image file no larger than 16 MiB"));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options
        .open(path)
        .map_err(|_| invalid("Cannot open local image"))?;
    if !file.metadata().is_ok_and(|m| m.is_file()) {
        return Err(invalid("Choose a regular image file"));
    }
    bounded_read(file)
}
fn validate_svg(bytes: &[u8]) -> Result<()> {
    use quick_xml::events::Event;
    let text = std::str::from_utf8(bytes).map_err(|_| invalid("Invalid SVG encoding"))?;
    let mut reader = quick_xml::Reader::from_str(text);
    let mut depth = 0usize;
    let mut root = false;
    loop {
        let event = reader
            .read_event()
            .map_err(|_| invalid("Invalid SVG document"))?;
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(e) | Event::Empty(e) => {
                // Restrict imports to self-contained SVG graphics. Uploaded
                // assets still use the existing sandboxed serving policy.
                let name = e.name();
                let tag = std::str::from_utf8(name.as_ref()).unwrap_or("");
                if ![
                    "svg",
                    "g",
                    "path",
                    "rect",
                    "circle",
                    "ellipse",
                    "line",
                    "polyline",
                    "polygon",
                    "defs",
                    "linearGradient",
                    "radialGradient",
                    "stop",
                    "clipPath",
                    "mask",
                    "use",
                    "title",
                    "desc",
                    "text",
                    "tspan",
                ]
                .contains(&tag)
                {
                    return Err(invalid("SVG imports must contain self-contained graphics"));
                }
                if depth == 0 {
                    if root || tag != "svg" {
                        return Err(invalid("Invalid SVG root"));
                    }
                    root = true;
                }
                for attr in e.attributes() {
                    let attr = attr.map_err(|_| invalid("Invalid SVG attribute"))?;
                    let key = std::str::from_utf8(attr.key.as_ref())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    let value = attr
                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                        .map_err(|_| invalid("Invalid SVG attribute"))?;
                    let lower = value.to_ascii_lowercase();
                    if key.starts_with("on")
                        || ((key == "href" || key.ends_with(":href")) && !value.starts_with('#'))
                        || (lower.contains("url(")
                            && !lower
                                .split("url(")
                                .skip(1)
                                .all(|v| v.starts_with('#') && v.contains(')')))
                        || key == "style"
                        || key == "xml:base"
                    {
                        return Err(invalid(
                            "SVG imports cannot reference external content or scripts",
                        ));
                    }
                }
                // Empty elements do not change depth.
                if !empty {
                    depth += 1;
                    if depth > 64 {
                        return Err(invalid("SVG nesting exceeds the limit"));
                    }
                }
            }
            Event::End(_) => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| invalid("Invalid SVG nesting"))?;
            }
            Event::CData(_) if depth == 0 => return Err(invalid("Invalid SVG document")),
            Event::Decl(_) if root => return Err(invalid("Invalid SVG document")),
            Event::GeneralRef(reference) => {
                let predefined =
                    [b"amp".as_slice(), b"lt", b"gt", b"apos", b"quot"].contains(&&*reference);
                let character = reference
                    .resolve_char_ref()
                    .map_err(|_| invalid("Invalid SVG character reference"))?
                    .is_some();
                if depth == 0 || (!predefined && !character) {
                    return Err(invalid("SVG imports cannot reference declared entities"));
                }
            }
            Event::DocType(_) | Event::PI(_) => {
                return Err(invalid(
                    "SVG imports cannot contain entity declarations or processing instructions",
                ))
            }
            Event::Text(t) if depth == 0 && !t.iter().all(u8::is_ascii_whitespace) => {
                return Err(invalid("Invalid SVG document"))
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !root || depth != 0 {
        return Err(invalid("Invalid SVG document"));
    }
    Ok(())
}
fn validated(bytes: Vec<u8>) -> Result<(String, Vec<u8>)> {
    if let Ok(format) = image::guess_format(&bytes) {
        let extension = match format {
            image::ImageFormat::Png => "png",
            image::ImageFormat::Jpeg => "jpg",
            image::ImageFormat::WebP => "webp",
            image::ImageFormat::Gif => "gif",
            _ => return Err(invalid("Use PNG, JPEG, WebP, GIF, or SVG images")),
        };
        let mut reader = image::ImageReader::with_format(Cursor::new(&bytes), format);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(8192);
        limits.max_image_height = Some(8192);
        limits.max_alloc = Some(64 * 1024 * 1024);
        reader.limits(limits);
        reader
            .decode()
            .map_err(|_| invalid("Invalid image or image dimensions exceed the limit"))?;
        return Ok((extension.into(), bytes));
    }
    validate_svg(&bytes)?;
    Ok(("svg".into(), bytes))
}
fn source_path(assets: &Assets, id: &str) -> Result<PathBuf> {
    assets.path(id)?;
    let root = assets.root.join("image_sources");
    if fs::symlink_metadata(&root).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(invalid("Invalid image source directory"));
    }
    let path = root.join(format!("{id}.json"));
    if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(invalid("Invalid image source"));
    }
    Ok(path)
}
pub fn import(assets: &Assets, request: ImageImport) -> Result<FileSource> {
    let (bytes, live_url) = match request {
        ImageImport::Url { url, live } => (download(&url)?, live.unwrap_or(false).then_some(url)),
        ImageImport::Local { path } => (local_bytes(&path)?, None),
    };
    let (extension, bytes) = validated(bytes)?;
    let source = assets.upload(&extension, &bytes)?;
    if let (FileSource::Asset { id }, Some(url)) = (&source, live_url) {
        let json = serde_json::to_vec(&url).map_err(|_| Error::execution())?;
        let path = source_path(assets, id)?;
        fs::create_dir_all(path.parent().ok_or_else(Error::execution)?)
            .map_err(|_| Error::execution())?;
        protected_write(&path, &json)?;
    }
    Ok(source)
}
pub fn is_live(assets: &Assets, id: &str) -> bool {
    source_path(assets, id).is_ok_and(|p| p.is_file())
}
pub fn refresh(assets: &Assets, id: &str) -> Result<FileSource> {
    refresh_using(assets, id, download)
}
fn refresh_using(
    assets: &Assets,
    id: &str,
    fetch: impl Fn(&str) -> Result<Vec<u8>>,
) -> Result<FileSource> {
    let path = source_path(assets, id)?;
    let bytes = local_bytes(
        path.to_str()
            .ok_or_else(|| invalid("Invalid image source"))?,
    )?;
    let url: String =
        serde_json::from_slice(&bytes).map_err(|_| invalid("This image has no live URL"))?;
    let (extension, bytes) = validated(fetch(&url)?)?;
    if id.rsplit('.').next() != Some(extension.as_str()) {
        return Err(invalid(
            "Live image format changed; import the new image separately",
        ));
    }
    atomic_replace(&assets.path(id)?, &bytes)?;
    Ok(FileSource::Asset { id: id.into() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{net::TcpListener, thread};
    const SVG: &[u8] = b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'><path d='M0 0h24v24z'/></svg>";
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let path = std::env::temp_dir()
                .join(format!("webdeck-images-{}", crate::domain::id().unwrap()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn assets(&self) -> Assets {
            Assets {
                root: self.0.clone(),
            }
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn nonpublic_destinations_and_credentialed_urls_are_rejected() {
        for ip in [
            "127.0.0.1",
            "10.0.0.1",
            "172.16.0.1",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "192.0.2.1",
            "198.18.0.1",
            "224.0.0.1",
            "240.0.0.1",
            "::1",
            "::ffff:127.0.0.1",
            "fc00::1",
            "fe80::1",
            "64:ff9b::7f00:1",
            "2001:db8::1",
            "2002:7f00:1::1",
        ] {
            assert!(!public_ip(ip.parse().unwrap()), "{ip}");
        }
        assert!(public_ip("8.8.8.8".parse().unwrap()));
        assert!(public_ip("2606:4700:4700::1111".parse().unwrap()));
        for url in [
            "file:///etc/passwd",
            "ftp://example.com/a.png",
            "https://user:password@example.com/a.png",
        ] {
            assert!(parse_url(url).is_err());
        }
        for url in [
            "http://127.0.0.1/a.png",
            "http://2130706433/a.png",
            "http://[::1]/a.png",
        ] {
            assert!(destination(&parse_url(url).unwrap()).is_err());
        }
    }
    #[test]
    fn imports_validate_content_and_copy_files_without_changing_configuration() {
        let temp = Temp::new();
        let path = temp.0.join("misleading.txt");
        fs::write(&path, SVG).unwrap();
        let result = import(
            &temp.assets(),
            ImageImport::Local {
                path: path.to_string_lossy().into(),
            },
        )
        .unwrap();
        let FileSource::Asset { id } = result else {
            panic!()
        };
        assert!(id.ends_with(".svg"));
        fs::remove_file(path).unwrap();
        assert_eq!(temp.assets().read(&id).unwrap(), SVG);
        assert!(!is_live(&temp.assets(), &id));
        assert!(local_bytes("relative.png").is_err());
        assert!(local_bytes(temp.0.to_str().unwrap()).is_err());
        assert!(validated(b"<html>not an image</html>".to_vec()).is_err());
        assert!(validated(b"\x89PNG\r\n\x1a\ninvalid".to_vec()).is_err());
        assert!(bounded_read(std::io::repeat(0).take(MAX_BYTES + 1)).is_err());
    }
    #[test]
    fn raster_validation_accepts_real_images_and_enforces_dimensions() {
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(24, 24)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        assert_eq!(validated(bytes.into_inner()).unwrap().0, "png");
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(9000, 1)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        assert!(validated(bytes.into_inner()).is_err());
    }
    #[test]
    fn svg_validation_rejects_active_external_and_malformed_content() {
        assert!(validate_svg(SVG).is_ok());
        assert!(validate_svg(b"<svg/>").is_ok());
        assert!(validate_svg(b"<svg><title>A &amp; B &#65;</title></svg>").is_ok());
        for text in [
            "<svg><script>alert(1)</script></svg>",
            "<svg onload='alert(1)'/>",
            "<svg><use href='https://example.com/x.svg'/></svg>",
            "<svg><path fill='url(https://example.com/x)'/></svg>",
            "<!DOCTYPE svg [<!ENTITY e SYSTEM 'file:///etc/passwd'>]><svg>&e;</svg>",
            "<svg><foreignObject/></svg>",
            "<svg><path></svg>",
            "<svg/><svg/>",
            "<svg>",
            "<svg><path style='fill:red'/></svg>",
        ] {
            assert!(validate_svg(text.as_bytes()).is_err(), "{text}");
        }
    }
    #[test]
    fn downloader_pins_addresses_and_revalidates_every_redirect() {
        use std::io::Write;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let task = thread::spawn(move || {
            for location in [Some("/image"), None, Some("http://127.0.0.1/private")] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0u8; 4096];
                let read = stream.read(&mut request).unwrap();
                assert!(read > 0);
                let response = if let Some(location) = location {
                    format!("HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                } else {
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        SVG.len(),
                        std::str::from_utf8(SVG).unwrap()
                    )
                };
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        let resolve = |url: &reqwest::Url| {
            if url.host_str() == Some("images.example") {
                Ok(("images.example".into(), vec![address]))
            } else {
                destination(url)
            }
        };
        assert_eq!(
            download_using(
                &format!("http://images.example:{}/start", address.port()),
                resolve
            )
            .unwrap(),
            SVG
        );
        assert!(download_using(
            &format!("http://images.example:{}/blocked", address.port()),
            resolve
        )
        .unwrap_err()
        .message
        .contains("public internet"));
        task.join().unwrap();
    }
    #[test]
    fn live_refresh_preserves_id_and_last_good_copy_on_failure() {
        let temp = Temp::new();
        let assets = temp.assets();
        let FileSource::Asset { id } = assets.upload("svg", SVG).unwrap() else {
            panic!()
        };
        let metadata = source_path(&assets, &id).unwrap();
        fs::create_dir_all(metadata.parent().unwrap()).unwrap();
        fs::write(metadata, b"\"https://images.example/icon.svg\"").unwrap();
        assert!(is_live(&assets, &id));
        let updated = b"<svg><circle r='5'/></svg>";
        assert_eq!(
            refresh_using(&assets, &id, |url| {
                assert_eq!(url, "https://images.example/icon.svg");
                Ok(updated.to_vec())
            })
            .unwrap(),
            FileSource::Asset { id: id.clone() }
        );
        assert_eq!(assets.read(&id).unwrap(), updated);
        assert!(refresh_using(&assets, &id, |_| Err(invalid("Offline"))).is_err());
        assert_eq!(assets.read(&id).unwrap(), updated);
        assert!(refresh_using(&assets, &id, |_| Ok(b"<html/>".to_vec())).is_err());
        assert_eq!(assets.read(&id).unwrap(), updated);
        assert!(refresh(&assets, "../secret").is_err());
    }
}
