//! Client for the control socket of a running Sonora, the same one its `sonora` command line
//! talks to (`crates/cli/src/wire.rs` in sonorahq/sonora, protocol 2).
//!
//! One exchange is one JSON line each way: `{"v":2,"body":<request>}` out and
//! `{"v":2,"body":<reply>}` back, where the reply body is `"done"`, `{"value":...}` or
//! `{"failed":...}`. Only Unix sockets are implemented; the Windows named pipe comes later.
//!
//! Plugins reach it through three operations, one per kind of request, so each operation maps
//! to one grant: `sonora.query` (read), `sonora.control` (audio) and `sonora.remote` (network).

use crate::{
    contracts::{Capability, ErrorCode},
    domain::{Error, Result},
    executor::Context,
};
use serde_json::{json, Value};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    time::Duration,
};

#[cfg(unix)]
use std::{
    io::{BufRead, BufReader, ErrorKind, Read, Write},
    os::unix::net::UnixStream,
};

/// The protocol both ends speak. Sonora refuses any other version.
pub const PROTOCOL_VERSION: u32 = 2;
/// Requests are a few words or a few queue items; anything larger is a mistake or an attack.
pub const MAX_REQUEST_BYTES: usize = 64 * 1024;
/// Queue snapshots (100 + 100 tracks) and 500 lines of lyrics fit well inside this.
pub const MAX_REPLY_BYTES: usize = 1024 * 1024;
/// Sonora's own limit for an answer, so a provider sign-in or lookup is not cut short.
pub const ANSWER_TIMEOUT: Duration = Duration::from_secs(60);

const SOCKET: &str = "sonora.sock";
const SOCKET_DEV: &str = "sonora-dev.sock";

/// What the running Sonora answered.
#[derive(Clone, Debug, PartialEq)]
pub enum Reply {
    /// The command was carried out and has nothing to report.
    Done,
    /// The value the request asked for.
    Value(Value),
    /// The request was refused. The body is Sonora's reason, as it sent it.
    Failed(Value),
}

/// Where Sonora's socket lives for a runtime folder, falling back to the cache folder the way
/// Sonora does when there is no runtime folder. `dev` picks the socket of a debug build.
pub fn socket_path(
    runtime_dir: Option<&OsStr>,
    cache_dir: Option<&Path>,
    dev: bool,
) -> Result<PathBuf> {
    let name = if dev { SOCKET_DEV } else { SOCKET };
    if let Some(runtime) = runtime_dir.filter(|dir| !dir.is_empty()) {
        return Ok(Path::new(runtime).join(name));
    }
    cache_dir
        .map(|cache| cache.join("sonora").join(name))
        .ok_or_else(|| {
            Error::new(
                ErrorCode::UnsupportedPlatform,
                "No runtime or cache folder to find Sonora's socket in",
            )
        })
}

/// The release socket of Sonora for this account, from the environment.
pub fn default_socket_path() -> Result<PathBuf> {
    if !cfg!(all(unix, not(target_os = "macos"))) {
        return Err(Error::new(
            ErrorCode::UnsupportedPlatform,
            "Sonora's control socket is not supported on this platform yet",
        ));
    }
    let runtime = std::env::var_os("XDG_RUNTIME_DIR");
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")));
    socket_path(runtime.as_deref(), cache.as_deref(), false)
}

/// Encodes a request as the line Sonora reads. Refuses what a plugin must not send: the
/// streaming `watch` (it needs its own connection lifetime) and local file paths (a plugin
/// must not make Sonora play arbitrary files from disk).
pub fn encode_request(body: &Value) -> Result<Vec<u8>> {
    let object = body.as_object();
    if object.is_some_and(|map| map.contains_key("watch")) {
        return Err(Error::new(
            ErrorCode::InvalidInput,
            "Streaming requests are not available to plugins",
        ));
    }
    if let Some(queue) = object
        .and_then(|map| map.get("queue"))
        .and_then(Value::as_object)
    {
        let has_path = queue
            .values()
            .filter_map(Value::as_array)
            .flatten()
            .any(|item| item.get("path").is_some());
        if has_path {
            return Err(Error::new(
                ErrorCode::InvalidInput,
                "Local file paths cannot be queued by a plugin",
            ));
        }
    }
    let mut line = serde_json::to_vec(&json!({"v": PROTOCOL_VERSION, "body": body}))
        .map_err(|_| Error::invalid())?;
    line.push(b'\n');
    if line.len() > MAX_REQUEST_BYTES {
        return Err(Error::new(ErrorCode::InvalidInput, "Request is too large"));
    }
    Ok(line)
}

/// Decodes one reply line, refusing another protocol version before reading the body.
pub fn decode_reply(line: &[u8]) -> Result<Reply> {
    let envelope: Value = serde_json::from_slice(line).map_err(|_| malformed())?;
    let version = envelope
        .get("v")
        .and_then(Value::as_u64)
        .ok_or_else(malformed)?;
    if version != u64::from(PROTOCOL_VERSION) {
        return Err(Error::new(
            ErrorCode::UnsupportedSchema,
            format!("Sonora speaks protocol {version}, WebDeck expects {PROTOCOL_VERSION}"),
        ));
    }
    match envelope.get("body") {
        Some(Value::String(word)) if word == "done" => Ok(Reply::Done),
        Some(Value::Object(body)) => {
            if let Some(value) = body.get("value") {
                Ok(Reply::Value(value.clone()))
            } else if let Some(failure) = body.get("failed") {
                Ok(Reply::Failed(failure.clone()))
            } else {
                Err(malformed())
            }
        }
        _ => Err(malformed()),
    }
}

/// A sentence for a refusal, for the person reading the plugin's error.
pub fn describe_failure(failure: &Value) -> String {
    match failure {
        Value::String(word) => match word.as_str() {
            "malformed" => "Sonora rejected the request as malformed".into(),
            "no-current-track" => "No track is playing in Sonora".into(),
            other => format!("Sonora refused the request ({other})"),
        },
        Value::Object(map) => {
            if let Some(version) = map.get("version") {
                format!("Sonora speaks protocol {version}, WebDeck expects {PROTOCOL_VERSION}")
            } else if let Some(Value::Object(detail)) = map.get("other-provider") {
                format!(
                    "That link belongs to {}, but {} is the current provider",
                    text(detail.get("link")),
                    text(detail.get("current")),
                )
            } else {
                "Sonora refused the request".into()
            }
        }
        _ => "Sonora refused the request".into(),
    }
}

fn text(value: Option<&Value>) -> &str {
    value.and_then(Value::as_str).unwrap_or("another provider")
}

fn malformed() -> Error {
    Error::new(
        ErrorCode::ExecutionFailed,
        "Sonora sent a reply that WebDeck does not understand",
    )
}

/// Sends one request to the Sonora listening on `path` and waits up to `timeout` for the
/// answer. The request is checked before anything is connected, and the reply is read with a
/// size limit, so a misbehaving peer cannot make WebDeck allocate without bound.
#[cfg(unix)]
pub fn call(path: &Path, body: &Value, timeout: Duration) -> Result<Reply> {
    let request = encode_request(body)?;
    let stream = UnixStream::connect(path).map_err(|error| connect_error(path, error))?;
    let timeout = timeout.max(Duration::from_millis(1));
    stream
        .set_read_timeout(Some(timeout))
        .and_then(|()| stream.set_write_timeout(Some(timeout)))
        .map_err(|_| Error::execution())?;
    (&stream).write_all(&request).map_err(io_error)?;

    let mut reader = BufReader::new((&stream).take(MAX_REPLY_BYTES as u64 + 1));
    let mut line = Vec::new();
    reader.read_until(b'\n', &mut line).map_err(io_error)?;
    if line.len() > MAX_REPLY_BYTES {
        return Err(Error::new(
            ErrorCode::ExecutionFailed,
            "Sonora's reply is too large",
        ));
    }
    if !line.ends_with(b"\n") {
        return Err(Error::new(
            ErrorCode::ExecutionFailed,
            "Sonora closed the connection without answering",
        ));
    }
    decode_reply(&line)
}

/// Other platforms have no socket client yet. The request is still checked, so the refusal
/// reads the same everywhere.
#[cfg(not(unix))]
pub fn call(_path: &Path, body: &Value, _timeout: Duration) -> Result<Reply> {
    encode_request(body)?;
    Err(Error::new(
        ErrorCode::UnsupportedPlatform,
        "Sonora's control socket is not supported on this platform yet",
    ))
}

/// Says which of the two situations this is, and where it looked. A missing socket means Sonora
/// is not open (or runs with another runtime folder); a socket nobody listens on is left over
/// from a run that ended without cleaning up.
#[cfg(unix)]
fn connect_error(path: &Path, error: std::io::Error) -> Error {
    let shown = path.display();
    match error.kind() {
        ErrorKind::NotFound => Error::new(
            ErrorCode::ExecutionFailed,
            format!("Sonora is not running: no socket at {shown}"),
        ),
        ErrorKind::ConnectionRefused => Error::new(
            ErrorCode::ExecutionFailed,
            format!("Sonora is not running: the socket at {shown} is left over, so restart Sonora"),
        ),
        _ => Error::new(
            ErrorCode::ExecutionFailed,
            format!("Could not reach Sonora at {shown}"),
        ),
    }
}

#[cfg(unix)]
fn io_error(error: std::io::Error) -> Error {
    match error.kind() {
        ErrorKind::WouldBlock | ErrorKind::TimedOut => {
            Error::new(ErrorCode::ExecutionFailed, "Sonora did not answer in time")
        }
        _ => Error::new(
            ErrorCode::ExecutionFailed,
            "The connection to Sonora failed",
        ),
    }
}

/// Runs one plugin operation. `input` is `{"request": <body>}`. The body must belong to the
/// operation, the grant for that operation is checked here, and Sonora's reply is turned into a
/// value or an error the plugin can show.
pub fn invoke(
    socket: Option<&Path>,
    operation: &str,
    input: &Value,
    context: &Context,
) -> Result<Value> {
    let request = input.get("request").ok_or_else(Error::invalid)?;
    let capability = check_operation(operation, request)?;
    if operation == "sonora.remote" && starts_playback(request) {
        context.check(Capability::Audio)?;
    }
    let timeout = context.remaining(capability, ANSWER_TIMEOUT)?;
    let path = match socket {
        Some(path) => path.to_path_buf(),
        None => default_socket_path()?,
    };
    match call(&path, request, timeout)? {
        Reply::Done => Ok(json!({})),
        Reply::Value(value) => Ok(value),
        Reply::Failed(failure) => Err(Error::new(
            ErrorCode::ExecutionFailed,
            describe_failure(&failure),
        )),
    }
}

/// The grant an operation needs, if `request` is a request of that operation's kind. Reads are
/// `sonora.query`, changes to playback are `sonora.control`, and requests that fetch from a
/// provider are `sonora.remote`. Anything else is refused, so a read-only grant can never send a
/// playback change.
pub fn check_operation(operation: &str, request: &Value) -> Result<Capability> {
    let (capability, accepted) = match operation {
        "sonora.query" => (Capability::Read, is_query(request)),
        "sonora.control" => (Capability::Audio, is_control(request)),
        "sonora.remote" => (Capability::Network, is_remote(request)),
        _ => return Err(Error::invalid()),
    };
    if accepted {
        Ok(capability)
    } else {
        Err(Error::new(
            ErrorCode::InvalidInput,
            "This request does not belong to that Sonora operation",
        ))
    }
}

/// The only key of an object, with its value. Every Sonora command has exactly one.
fn single(value: &Value) -> Option<(&str, &Value)> {
    let map = value.as_object()?;
    if map.len() != 1 {
        return None;
    }
    map.iter().next().map(|(key, value)| (key.as_str(), value))
}

/// Requests that read state and change nothing.
fn is_query(request: &Value) -> bool {
    const STATE: [&str; 4] = ["volume", "shuffle", "mute", "repeat"];
    if request.as_str() == Some("lyrics") {
        return true;
    }
    match single(request) {
        Some(("playback", Value::String(word))) => word == "current-track",
        Some(("playback", value)) => {
            matches!(single(value), Some((key, Value::Null)) if STATE.contains(&key))
        }
        Some(("queue", value)) => value.as_str() == Some("snapshot"),
        Some(("like", value)) => value.is_null(),
        Some(("provider", value)) => {
            value.as_str() == Some("list")
                || matches!(single(value), Some(("current", Value::Null)))
        }
        Some(("session", value)) => value.as_str() == Some("status"),
        _ => false,
    }
}

/// Changes to what plays, and to the provider in use. Nothing here fetches from a provider.
fn is_control(request: &Value) -> bool {
    match single(request) {
        Some(("playback", Value::String(verb))) => {
            matches!(
                verb.as_str(),
                "play" | "pause" | "toggle" | "next" | "previous"
            )
        }
        Some(("playback", value)) => match single(value) {
            Some(("seek", percent)) => percent.as_u64().is_some_and(|percent| percent <= 100),
            Some(("volume", level)) => {
                matches!(single(level), Some(("to" | "by", Value::Number(_))))
            }
            Some(("mute" | "shuffle", switch)) => {
                matches!(switch.as_str(), Some("on" | "off" | "toggle"))
            }
            Some(("repeat", mode)) => matches!(mode.as_str(), Some("off" | "all" | "one")),
            _ => false,
        },
        Some(("like", switch)) => matches!(switch.as_str(), Some("on" | "off" | "toggle")),
        Some(("provider", value)) => {
            matches!(single(value), Some(("current", slug)) if slug.is_string())
        }
        _ => false,
    }
}

/// Searches and queue additions that Sonora resolves with the provider.
fn is_remote(request: &Value) -> bool {
    match single(request) {
        Some(("search", query)) => query.is_object(),
        Some(("queue", value)) => match single(value) {
            Some(("add" | "play-next" | "play", items)) => items.as_array().is_some_and(|items| {
                !items.is_empty() && items.len() <= 16 && items.iter().all(is_provider_item)
            }),
            _ => false,
        },
        _ => false,
    }
}

fn is_provider_item(item: &Value) -> bool {
    match single(item) {
        Some(("link", link)) => link.is_string(),
        Some(("search", query)) => query.is_object(),
        _ => false,
    }
}

fn starts_playback(request: &Value) -> bool {
    matches!(single(request), Some(("queue", value)) if single(value).is_some_and(|(key, _)| key == "play"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_lives_in_the_runtime_folder_or_falls_back_to_cache() {
        assert_eq!(
            socket_path(Some(OsStr::new("/run/user/1000")), None, false).unwrap(),
            PathBuf::from("/run/user/1000/sonora.sock")
        );
        assert_eq!(
            socket_path(Some(OsStr::new("/run/user/1000")), None, true).unwrap(),
            PathBuf::from("/run/user/1000/sonora-dev.sock")
        );
        assert_eq!(
            socket_path(
                Some(OsStr::new("")),
                Some(Path::new("/home/a/.cache")),
                false
            )
            .unwrap(),
            PathBuf::from("/home/a/.cache/sonora/sonora.sock")
        );
        assert_eq!(
            socket_path(None, None, false).unwrap_err().code,
            ErrorCode::UnsupportedPlatform
        );
    }

    #[test]
    fn requests_are_encoded_in_the_v2_envelope() {
        assert_eq!(
            encode_request(&json!("lyrics")).unwrap(),
            b"{\"v\":2,\"body\":\"lyrics\"}\n".to_vec()
        );
        assert_eq!(
            encode_request(&json!({"playback": {"volume": {"to": 40}}})).unwrap(),
            b"{\"v\":2,\"body\":{\"playback\":{\"volume\":{\"to\":40}}}}\n".to_vec()
        );
    }

    #[test]
    fn streaming_requests_are_refused() {
        let error = encode_request(&json!({"watch": {"position": false}})).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn local_file_paths_are_refused_in_every_queue_command() {
        for command in ["add", "play-next", "play"] {
            let body = json!({"queue": {command: [{"link": "https://example.test/a"}, {"path": "/etc/passwd"}]}});
            assert_eq!(
                encode_request(&body).unwrap_err().code,
                ErrorCode::InvalidInput,
                "{command}"
            );
        }
        assert!(
            encode_request(&json!({"queue": {"add": [{"link": "https://example.test/a"}]}}))
                .is_ok()
        );
    }

    #[test]
    fn oversized_requests_are_refused() {
        let body =
            json!({"search": {"text": "a".repeat(MAX_REQUEST_BYTES), "kind": null, "limit": 1}});
        assert_eq!(
            encode_request(&body).unwrap_err().code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn replies_are_decoded_and_other_versions_refused() {
        assert_eq!(
            decode_reply(b"{\"v\":2,\"body\":\"done\"}\n").unwrap(),
            Reply::Done
        );
        assert_eq!(
            decode_reply(b"{\"v\":2,\"body\":{\"value\":{\"volume\":70}}}\n").unwrap(),
            Reply::Value(json!({"volume": 70}))
        );
        assert_eq!(
            decode_reply(b"{\"v\":2,\"body\":{\"failed\":\"no-current-track\"}}\n").unwrap(),
            Reply::Failed(json!("no-current-track"))
        );
        assert_eq!(
            decode_reply(b"{\"v\":3,\"body\":\"done\"}\n")
                .unwrap_err()
                .code,
            ErrorCode::UnsupportedSchema
        );
        assert_eq!(
            decode_reply(b"{\"v\":2}\n").unwrap_err().code,
            ErrorCode::ExecutionFailed
        );
        assert_eq!(
            decode_reply(b"not json\n").unwrap_err().code,
            ErrorCode::ExecutionFailed
        );
    }

    #[test]
    fn each_operation_accepts_only_its_own_kind_of_request() {
        let query = [
            json!("lyrics"),
            json!({"playback": "current-track"}),
            json!({"playback": {"volume": null}}),
            json!({"queue": "snapshot"}),
            json!({"like": null}),
            json!({"provider": "list"}),
            json!({"provider": {"current": null}}),
            json!({"session": "status"}),
        ];
        let control = [
            json!({"playback": "next"}),
            json!({"playback": {"seek": 50}}),
            json!({"playback": {"volume": {"to": 40}}}),
            json!({"playback": {"mute": "toggle"}}),
            json!({"playback": {"repeat": "all"}}),
            json!({"like": "on"}),
            json!({"provider": {"current": "spotify"}}),
        ];
        let remote = [
            json!({"search": {"text": "daft punk", "kind": "song", "limit": 3}}),
            json!({"queue": {"add": [{"link": "https://example.test/a"}]}}),
            json!({"queue": {"play": [{"search": {"text": "x", "kind": "song"}}]}}),
        ];
        for request in &query {
            assert_eq!(
                check_operation("sonora.query", request).unwrap(),
                Capability::Read,
                "{request}"
            );
        }
        for request in &control {
            assert_eq!(
                check_operation("sonora.control", request).unwrap(),
                Capability::Audio,
                "{request}"
            );
        }
        for request in &remote {
            assert_eq!(
                check_operation("sonora.remote", request).unwrap(),
                Capability::Network,
                "{request}"
            );
        }
        // Each kind is refused by the other two operations.
        for request in query.iter().chain(&control).chain(&remote) {
            let accepted = ["sonora.query", "sonora.control", "sonora.remote"]
                .iter()
                .filter(|op| check_operation(op, request).is_ok())
                .count();
            assert_eq!(accepted, 1, "{request}");
        }
        // Refused by all three operations.
        let refused = [
            json!({"playback": {"seek-time": {"to": 1}}}),
            json!({"queue": {"add": []}}),
            json!({"queue": {"add": [{"path": "/etc/passwd"}]}}),
            json!({"watch": {"position": false}}),
            json!({"search": "not an object"}),
        ];
        for request in &refused {
            for operation in ["sonora.query", "sonora.control", "sonora.remote"] {
                assert!(
                    check_operation(operation, request).is_err(),
                    "{operation} {request}"
                );
            }
        }
        // Read forms are not accepted as control, and control verbs are not reads.
        assert!(check_operation("sonora.control", &json!({"playback": {"volume": null}})).is_err());
        assert!(check_operation("sonora.control", &json!({"playback": "current-track"})).is_err());
        assert!(check_operation("sonora.control", &json!({"queue": "snapshot"})).is_err());
        assert!(check_operation("sonora.query", &json!({"playback": "play"})).is_err());
        assert!(check_operation("sonora.remote", &json!({"playback": "play"})).is_err());
        assert!(check_operation("sonora.unknown", &json!("lyrics")).is_err());
    }

    #[test]
    fn failures_have_readable_descriptions() {
        assert_eq!(
            describe_failure(&json!("no-current-track")),
            "No track is playing in Sonora"
        );
        assert_eq!(
            describe_failure(&json!({"version": 2})),
            "Sonora speaks protocol 2, WebDeck expects 2"
        );
        assert_eq!(
            describe_failure(
                &json!({"other-provider": {"link": "Spotify", "current": "YouTube Music"}})
            ),
            "That link belongs to Spotify, but YouTube Music is the current provider"
        );
        assert_eq!(
            describe_failure(&json!({"unknown": 1})),
            "Sonora refused the request"
        );
    }
}

#[cfg(all(test, unix))]
mod socket_tests {
    use super::*;
    use std::{
        os::unix::net::UnixListener,
        thread,
        time::{SystemTime, UNIX_EPOCH},
    };

    /// A socket path under the temp folder, unique per test, removed when dropped.
    struct TestSocket(PathBuf);

    impl TestSocket {
        fn new(tag: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            Self(std::env::temp_dir().join(format!(
                "webdeck-sonora-{tag}-{}-{nanos}.sock",
                std::process::id()
            )))
        }
    }

    impl Drop for TestSocket {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    /// Binds before returning, so the client cannot race the server. The server reads one
    /// request line, then answers with `reply`. `None` means it waits until the client hangs up.
    fn serve(socket: &TestSocket, reply: Option<Vec<u8>>) -> thread::JoinHandle<String> {
        let listener = UnixListener::bind(&socket.0).unwrap();
        thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut request = String::new();
            BufReader::new(&stream).read_line(&mut request).unwrap();
            match reply {
                Some(bytes) => {
                    // The client may already have stopped reading, which is fine here.
                    let _ = (&stream).write_all(&bytes);
                }
                None => {
                    let mut rest = Vec::new();
                    let _ = (&stream).read_to_end(&mut rest);
                }
            }
            request
        })
    }

    const WAIT: Duration = Duration::from_secs(5);

    #[test]
    fn a_value_reply_is_returned_and_the_request_sent_in_the_envelope() {
        let socket = TestSocket::new("value");
        let server = serve(
            &socket,
            Some(b"{\"v\":2,\"body\":{\"value\":{\"volume\":70}}}\n".to_vec()),
        );
        let reply = call(&socket.0, &json!({"playback": {"volume": null}}), WAIT).unwrap();
        assert_eq!(reply, Reply::Value(json!({"volume": 70})));
        assert_eq!(
            server.join().unwrap(),
            "{\"v\":2,\"body\":{\"playback\":{\"volume\":null}}}\n"
        );
    }

    #[test]
    fn done_and_failed_replies_are_returned_as_such() {
        let socket = TestSocket::new("done");
        let server = serve(&socket, Some(b"{\"v\":2,\"body\":\"done\"}\n".to_vec()));
        assert_eq!(
            call(&socket.0, &json!({"playback": "next"}), WAIT).unwrap(),
            Reply::Done
        );
        server.join().unwrap();

        let socket = TestSocket::new("failed");
        let server = serve(
            &socket,
            Some(b"{\"v\":2,\"body\":{\"failed\":\"no-current-track\"}}\n".to_vec()),
        );
        assert_eq!(
            call(&socket.0, &json!({"like": null}), WAIT).unwrap(),
            Reply::Failed(json!("no-current-track"))
        );
        server.join().unwrap();
    }

    #[test]
    fn a_reply_of_another_protocol_version_is_refused() {
        let socket = TestSocket::new("version");
        let server = serve(&socket, Some(b"{\"v\":3,\"body\":\"done\"}\n".to_vec()));
        let error = call(&socket.0, &json!("lyrics"), WAIT).unwrap_err();
        assert_eq!(error.code, ErrorCode::UnsupportedSchema);
        server.join().unwrap();
    }

    #[test]
    fn a_silent_server_times_out_instead_of_hanging() {
        let socket = TestSocket::new("silent");
        let server = serve(&socket, None);
        let started = std::time::Instant::now();
        let error = call(&socket.0, &json!("lyrics"), Duration::from_millis(200)).unwrap_err();
        assert_eq!(error.message, "Sonora did not answer in time");
        assert!(started.elapsed() < Duration::from_secs(4));
        server.join().unwrap();
    }

    #[test]
    fn a_reply_over_the_limit_is_refused_without_reading_it_all() {
        let socket = TestSocket::new("huge");
        let mut reply = vec![b'a'; MAX_REPLY_BYTES + 10];
        reply.push(b'\n');
        let server = serve(&socket, Some(reply));
        let error = call(&socket.0, &json!("lyrics"), WAIT).unwrap_err();
        assert_eq!(error.message, "Sonora's reply is too large");
        server.join().unwrap();
    }

    #[test]
    fn a_server_that_hangs_up_without_answering_is_reported() {
        let socket = TestSocket::new("hangup");
        let server = serve(&socket, Some(Vec::new()));
        let error = call(&socket.0, &json!("lyrics"), WAIT).unwrap_err();
        assert_eq!(
            error.message,
            "Sonora closed the connection without answering"
        );
        server.join().unwrap();
    }

    #[test]
    fn a_garbled_reply_is_reported_as_not_understood() {
        let socket = TestSocket::new("garbled");
        let server = serve(&socket, Some(b"nope\n".to_vec()));
        let error = call(&socket.0, &json!("lyrics"), WAIT).unwrap_err();
        assert_eq!(error.code, ErrorCode::ExecutionFailed);
        server.join().unwrap();
    }

    #[test]
    fn a_missing_socket_means_sonora_is_not_running() {
        let socket = TestSocket::new("absent");
        let error = call(&socket.0, &json!("lyrics"), WAIT).unwrap_err();
        assert_eq!(
            error.message,
            format!("Sonora is not running: no socket at {}", socket.0.display())
        );
    }

    #[test]
    fn a_socket_nobody_listens_on_says_to_restart_sonora() {
        let socket = TestSocket::new("leftover");
        // Binding and dropping the listener leaves the socket file behind, as a crash does.
        drop(UnixListener::bind(&socket.0).unwrap());
        let error = call(&socket.0, &json!("lyrics"), WAIT).unwrap_err();
        assert!(
            error.message.contains("left over") && error.message.contains("restart Sonora"),
            "{}",
            error.message
        );
    }

    #[test]
    fn a_bad_request_is_refused_before_anything_connects() {
        // The socket does not exist, so a connection attempt would say "not running" instead.
        let socket = TestSocket::new("nobind");
        let error = call(&socket.0, &json!({"watch": {"position": false}}), WAIT).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput);
    }
}
