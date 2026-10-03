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
