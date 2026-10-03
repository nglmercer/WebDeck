//! Test fixture: a native plugin using napi-vm's independently embeddable SDK.
use napi_vm_plugin_sdk::{serve, Contract, Plugin, PluginMetadata, Registry};
#[tokio::main]
async fn main() {
    let contract = Contract::from_value(
        serde_json::from_str(include_str!("trusted-plugin/contract.json")).unwrap(),
    )
    .unwrap();
    let registry = Registry::new();
    registry
        .register(contract, "echo", |input, context| {
            Box::pin(async move {
                context.throw_if_cancelled()?;
                if input["text"] == "crash" {
                    std::process::exit(7);
                }
                if input["text"] == "slow" {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
                Ok(input)
            })
        })
        .unwrap();
    let plugin = Plugin::new(registry);
    let metadata = PluginMetadata {
        id: "fixture".into(),
        version: "2.0.0".into(),
        requires_host: vec![],
    };
    if serve(plugin, metadata).await.is_err() {
        std::process::exit(1);
    }
}
