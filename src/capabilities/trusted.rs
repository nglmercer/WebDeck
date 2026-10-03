use super::*;
use crate::runtime::plugins::{confined, RuntimePlugin};
use napi_vm_plugin_host::{Host, LoadOptions, PluginHandle};
#[derive(Default)]
pub(super) struct TrustedOwner(Mutex<Option<TrustedRuntime>>);
struct TrustedRuntime {
    runtime: tokio::runtime::Runtime,
    host: Host,
    plugins: std::collections::HashMap<String, PluginHandle>,
}
impl TrustedOwner {
    pub fn invoke(
        &self,
        plugin: &RuntimePlugin,
        action: &str,
        args: &Value,
        context: &Context,
    ) -> Result<Value> {
        let mut guard = self.0.lock().map_err(|_| Error::execution())?;
        if guard.is_none() {
            *guard = Some(TrustedRuntime {
                runtime: tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(1)
                    .enable_all()
                    .build()
                    .map_err(|_| Error::execution())?,
                host: Host::default(),
                plugins: std::collections::HashMap::new(),
            });
        }
        let owner = guard.as_mut().ok_or_else(Error::execution)?;
        let path = confined(&plugin.root, &plugin.manifest.entry)?;
        // Only executable manifests pass the WebDeck loader. Default host policy
        // verifies the declared package's artifacts before launching its native child.
        let remaining = context.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(Error::execution());
        }
        owner.runtime.block_on(async {
            tokio::time::timeout(remaining, async {
                if !owner.plugins.contains_key(&plugin.manifest.id) {
                    let loaded = owner
                        .host
                        .load(&path, LoadOptions::default())
                        .await
                        .map_err(|_| {
                            Error::new(ErrorCode::ExecutionFailed, "Trusted plugin startup failed")
                        })?;
                    owner.plugins.insert(plugin.manifest.id.clone(), loaded);
                }
                let handle = owner
                    .plugins
                    .get(&plugin.manifest.id)
                    .ok_or_else(Error::execution)?;
                let contract = handle
                    .contract(&plugin.manifest.contract)
                    .map_err(|_| Error::invalid())?;
                handle
                    .invoke(&contract, action, args.clone())
                    .await
                    .map_err(|_| {
                        Error::new(ErrorCode::ExecutionFailed, "Trusted plugin action failed")
                    })
            })
            .await
            .map_err(|_| {
                Error::new(
                    ErrorCode::ExecutionFailed,
                    "Trusted plugin timed out; execution outcome is unknown",
                )
            })?
        })
    }
    pub fn shutdown(&self) {
        if let Some(owner) = self.0.lock().unwrap_or_else(|p| p.into_inner()).take() {
            let _ = owner.runtime.block_on(owner.host.shutdown());
        }
    }
}
