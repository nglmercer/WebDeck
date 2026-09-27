//! Port of `app/utils/settings/save_config.py`.

use serde_json::Value;

use crate::app::utils::global_variables::set_global_variable;

/// Port of `save_config` — writes `.config/config.json`, syncs the
/// `config` global, and returns the config.
pub fn save_config(config: Value) -> Value {
    use crate::app::utils::settings::get_config::{get_config_path, write_config_atomically};
    let content = serde_json::to_string_pretty(&config).expect("Cannot serialize config");
    write_config_atomically(&get_config_path(), &content);
    set_global_variable("config", config.clone());
    config
}
