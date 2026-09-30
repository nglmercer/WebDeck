//! Port of `app/utils/settings/save_config.py`.

use serde_json::Value;

use crate::app::utils::global_variables::set_global_variable;

/// Port of `save_config` — writes `.config/config.json`, syncs the
/// `config` global, and returns the config.
pub fn save_config(config: Value) -> Value {
    use crate::app::utils::settings::get_config::save_config as persist;
    persist(&config);
    set_global_variable("config", config.clone());
    config
}
