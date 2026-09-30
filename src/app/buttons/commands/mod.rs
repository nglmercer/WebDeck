//! Compatibility facade. Parsing and policy live in domain/application;
//! platform effects are implemented by the native adapter.
use crate::domain::command::parse_legacy;
use serde_json::{json, Value};

pub fn handle_command(message: &str) -> Value {
    let plugins: Vec<String> = crate::app::utils::plugins::load_plugins::plugin_commands()
        .values()
        .flat_map(|p| p.keys().cloned())
        .collect();
    match parse_legacy(message, &plugins) {
        Ok(command) => {
            if !crate::application::executor::context_allows(command.capability) {
                return json!({"success": false, "message": "Capability denied"});
            }
            crate::adapters::platform::execute(&command)
        }
        Err(error) => json!({"success": false, "message": error.message}),
    }
}
