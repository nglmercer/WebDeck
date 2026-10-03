use super::VmRuntime;
use crate::{
    contracts::Command,
    domain::Result,
    executor::{Adapter, Context},
};
use serde_json::Value;
use std::sync::Arc;

pub struct VmAdapter {
    runtime: Arc<VmRuntime>,
}
impl VmAdapter {
    pub fn new(runtime: Arc<VmRuntime>) -> Self {
        Self { runtime }
    }
}
impl Adapter for VmAdapter {
    fn execute(&self, command: &Command, context: &Context) -> Result<Value> {
        self.runtime.invoke(command, context)
    }
    fn shutdown(&self) {
        self.runtime.shutdown();
    }
}
