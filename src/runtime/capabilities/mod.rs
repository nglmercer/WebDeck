use crate::{contracts::Capability, domain::Result, executor::Context};
use serde_json::Value;

/// Host implementations must honor the authorized context and remaining deadline.
/// No guest code receives the capability object itself.
pub trait Metrics: Send + Sync + 'static {
    fn usage(&self, context: &Context) -> Result<Value>;
}
pub struct NativeMetrics;
impl Metrics for NativeMetrics {
    fn usage(&self, context: &Context) -> Result<Value> {
        context.check(Capability::Read)?;
        let value = crate::native::usage();
        context.check(Capability::Read)?;
        Ok(value)
    }
}

/// Stable product primitives, never a raw Command dispatcher.
pub trait CapabilityHost: Send + Sync + 'static {
    fn call(&self, operation: &str, input: &Value, context: &Context) -> Result<Value>;
    fn shutdown(&self) {}
}
pub struct UnavailableHost;
impl CapabilityHost for UnavailableHost {
    fn call(&self, _: &str, _: &Value, _: &Context) -> Result<Value> {
        Err(crate::domain::Error::new(
            crate::contracts::ErrorCode::ExecutionFailed,
            "Host capability unavailable",
        ))
    }
}

pub fn required_capability(operation: &str) -> Result<Capability> {
    match operation {
        "network.fetch" => Ok(Capability::Network),
        "input.perform" => Ok(Capability::Input),
        "window.open"
        | "window.foreground"
        | "window.kill"
        | "process.spawn"
        | "window.restartDesktop"
        | "window.closeFocused"
        | "capture.pick" => Ok(Capability::Window),
        "system.power" | "system.exit" | "system.screensaver" | "system.screensaverSettings" => {
            Ok(Capability::Power)
        }
        "system.firewall" => Ok(Capability::Admin),
        "audio.volume" | "audio.appVolume" | "audio.media" | "audio.endpoint" | "audio.play"
        | "audio.stopAll" => Ok(Capability::Audio),
        "process.shell" | "storage.source" => Ok(Capability::Script),
        "storage.button" => Ok(Capability::Read),
        _ => Err(crate::domain::Error::invalid()),
    }
}
