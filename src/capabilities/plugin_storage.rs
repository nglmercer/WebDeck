use super::*;
impl Platform {
    pub(super) fn plugin_storage(
        &self,
        operation: &str,
        input: &Value,
        context: &Context,
    ) -> Result<Value> {
        let id = context
            .principal
            .as_deref()
            .filter(|id| {
                !id.starts_with("builtin.")
                    && !id.is_empty()
                    && id
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c))
                    && *id != "."
                    && *id != ".."
            })
            .ok_or_else(|| {
                Error::new(
                    ErrorCode::Forbidden,
                    "Plugin storage requires an isolated plugin identity",
                )
            })?;
        let key = input["key"]
            .as_str()
            .filter(|k| {
                !k.is_empty()
                    && k.len() <= 128
                    && !["__proto__", "constructor", "prototype"].contains(k)
            })
            .ok_or_else(Error::invalid)?;
        let directory = self.assets.root.join("plugin-state");
        if fs::symlink_metadata(&directory).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::invalid());
        }
        fs::create_dir_all(&directory).map_err(|_| Error::execution())?;
        let path = directory.join(format!("{id}.json"));
        if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::invalid());
        }
        let mut state: BTreeMap<String, Value> = if path.exists() {
            let mut bytes = Vec::new();
            fs::File::open(&path)
                .map_err(|_| Error::execution())?
                .take(65537)
                .read_to_end(&mut bytes)
                .map_err(|_| Error::execution())?;
            if bytes.len() > 65536 {
                return Err(Error::invalid());
            }
            serde_json::from_slice(&bytes).map_err(|_| Error::invalid())?
        } else {
            BTreeMap::new()
        };
        if operation == "storage.pluginGet" {
            return Ok(state.get(key).cloned().unwrap_or(Value::Null));
        }
        state.insert(
            key.to_string(),
            input.get("value").ok_or_else(Error::invalid)?.clone(),
        );
        let bytes = serde_json::to_vec(&state).map_err(|_| Error::invalid())?;
        if state.len() > 256 || bytes.len() > 65536 {
            return Err(Error::invalid());
        }
        crate::storage::atomic_replace(&path, &bytes)?;
        Ok(Value::Null)
    }
}
