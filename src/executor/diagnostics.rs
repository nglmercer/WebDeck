use crate::contracts::{Capability, ErrorCode};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    sync::{mpsc, OnceLock},
    time::Instant,
};

fn start_writer(
    capacity: usize,
    mut write: impl FnMut(Vec<u8>) + Send + 'static,
) -> std::io::Result<mpsc::SyncSender<Vec<u8>>> {
    let (sender, receiver) = mpsc::sync_channel(capacity);
    std::thread::Builder::new()
        .name("webdeck-diagnostics".into())
        .spawn(move || {
            for record in receiver {
                write(record);
            }
        })?;
    Ok(sender)
}

fn writer() -> Option<&'static mpsc::SyncSender<Vec<u8>>> {
    static WRITER: OnceLock<Option<mpsc::SyncSender<Vec<u8>>>> = OnceLock::new();
    WRITER
        .get_or_init(|| {
            start_writer(256, |record| {
                let _ = std::io::stderr().lock().write_all(&record);
            })
            .ok()
        })
        .as_ref()
}

/// Contains only allowlisted metadata; never receives an action payload or error message.
pub(super) struct Diagnostic {
    started: Instant,
    request_id_sha256: String,
    category: Capability,
    outcome: &'static str,
    error_code: Option<ErrorCode>,
}
#[derive(Serialize)]
struct Event<'a> {
    event: &'static str,
    request_id_sha256: &'a str,
    category: Capability,
    duration_ms: u128,
    outcome: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<ErrorCode>,
}
impl Diagnostic {
    pub(super) fn new(request_id: &str, category: Capability) -> Self {
        Self {
            started: Instant::now(),
            request_id_sha256: format!("{:x}", Sha256::digest(request_id.as_bytes())),
            category,
            outcome: "unknown",
            error_code: None,
        }
    }
    pub(super) fn finish(&mut self, outcome: &'static str, error_code: Option<ErrorCode>) {
        self.outcome = outcome;
        self.error_code = error_code;
    }
    fn event(&self) -> Event<'_> {
        Event {
            event: "command_terminal",
            request_id_sha256: &self.request_id_sha256,
            category: self.category,
            duration_ms: self.started.elapsed().as_millis(),
            outcome: self.outcome,
            error_code: self.error_code,
        }
    }
}
impl Drop for Diagnostic {
    fn drop(&mut self) {
        if std::env::var_os("WEBDECK_DIAGNOSTICS").as_deref() != Some(std::ffi::OsStr::new("1")) {
            return;
        }
        // Best-effort logging never waits for stderr or queue capacity.
        if let (Some(writer), Ok(mut record)) = (writer(), serde_json::to_vec(&self.event())) {
            record.push(b'\n');
            let _ = writer.try_send(record);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blocked_sink_has_bounded_queue_and_does_not_block_producers() {
        let (started, observe_started) = mpsc::channel();
        let (release, wait_release) = mpsc::channel();
        let (written, observe_written) = mpsc::channel();
        let writer = start_writer(1, move |record| {
            started.send(()).unwrap();
            wait_release.recv().unwrap();
            written.send(record).unwrap();
        })
        .unwrap();
        writer.try_send(vec![1]).unwrap();
        observe_started.recv().unwrap();
        writer.try_send(vec![2]).unwrap();
        assert!(matches!(
            writer.try_send(vec![3]),
            Err(mpsc::TrySendError::Full(_))
        ));
        release.send(()).unwrap();
        assert_eq!(observe_written.recv().unwrap(), vec![1]);
        observe_started.recv().unwrap();
        release.send(()).unwrap();
        assert_eq!(observe_written.recv().unwrap(), vec![2]);
        drop(writer);
    }
    #[test]
    fn allowlisted_event_hashes_untrusted_ids_and_omits_error_text() {
        let mut diagnostic = Diagnostic::new("secret-token\nscript source", Capability::Script);
        diagnostic.finish("failed", Some(ErrorCode::ExecutionFailed));
        let event = serde_json::to_value(diagnostic.event()).unwrap();
        assert_eq!(event["category"], "script");
        assert_eq!(event["outcome"], "failed");
        assert_eq!(event["request_id_sha256"].as_str().unwrap().len(), 64);
        assert_eq!(event.as_object().unwrap().len(), 6);
        let json = event.to_string();
        assert!(!json.contains("secret-token"));
        assert!(!json.contains("script source"));
        assert!(!json.contains("message"));
    }
}
