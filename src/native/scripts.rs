use super::*;
impl Native {
    pub(super) fn engine(self: &Arc<Self>, x: &Context) -> rhai::Engine {
        let mut e = rhai::Engine::new();
        e.set_max_operations(100000);
        e.set_max_call_levels(32);
        e.set_max_expr_depths(64, 32);
        e.set_max_string_size(65536);
        e.set_max_array_size(4096);
        e.set_max_map_size(1024);
        let deadline = x.deadline;
        e.on_progress(move |_| {
            if Instant::now() > deadline {
                Some("execution deadline".into())
            } else {
                None
            }
        });
        let native = self.clone();
        let context = x.nested();
        e.register_fn(
            "invoke",
            move |args: rhai::Map| -> std::result::Result<rhai::Dynamic, Box<rhai::EvalAltResult>> {
                let v: rhai::Dynamic = args.into();
                let c: Command = rhai::serde::from_dynamic(&v)
                    .map_err(|_| Box::<rhai::EvalAltResult>::from("invalid typed command"))?;
                let result = native.exec(&c, &context).map_err(|_| {
                    Box::<rhai::EvalAltResult>::from("nested command denied or failed")
                })?;
                rhai::serde::to_dynamic(result)
                    .map_err(|_| Box::<rhai::EvalAltResult>::from("invalid result"))
            },
        );
        e.on_print(|_| {});
        e
    }
    pub(super) fn script(self: &Arc<Self>, s: &ScriptSource, x: &Context) -> Result<Value> {
        let result = self
            .engine(x)
            .eval::<rhai::Dynamic>(&self.source(s)?)
            .map_err(|_| Error::execution())?;
        let value: rhai::Dynamic = result;
        let data: Value = rhai::serde::from_dynamic(&value).unwrap_or(Value::Null);
        Ok(json!({"value":data}))
    }
    pub(super) fn plugin(
        self: &Arc<Self>,
        id: &str,
        version: &str,
        action: &str,
        args: &BTreeMap<String, Value>,
        x: &Context,
    ) -> Result<Value> {
        let loaded = self
            .plugins
            .iter()
            .find(|p| p.manifest.id == id && p.manifest.version == version)
            .ok_or_else(Error::invalid)?;
        let m = &loaded.manifest;
        let a = m
            .actions
            .iter()
            .find(|a| a.id == action)
            .ok_or_else(Error::invalid)?;
        for c in &a.capabilities {
            x.check(*c)?;
        }
        for (k, v) in args {
            let field = a.arguments.get(k).ok_or_else(Error::invalid)?;
            validate_plugin_type(&field.r#type, v)?;
        }
        for (k, v) in &a.arguments {
            if v.required && !args.contains_key(k) {
                return Err(Error::invalid());
            }
        }
        let mut context = x.clone();
        context.capabilities.retain(|c| a.capabilities.contains(c));
        let e = self.engine(&context);
        let ast = &loaded.ast;
        let arg = rhai::serde::to_dynamic(args).map_err(|_| Error::invalid())?;
        let result = e
            .call_fn::<rhai::Dynamic>(
                &mut rhai::Scope::new(),
                ast,
                "invoke_action",
                (action.to_string(), arg),
            )
            .map_err(|_| Error::execution())?;
        let v: Value = rhai::serde::from_dynamic(&result).map_err(|_| Error::execution())?;
        validate_plugin_type(&a.result.r#type, &v)?;
        Ok(json!({"plugin_id":id,"version":version,"action_id":action,"value":v}))
    }
}
fn validate_plugin_type(t: &str, v: &Value) -> Result<()> {
    if match t {
        "string" => v.is_string(),
        "number" => v.is_number(),
        "boolean" => v.is_boolean(),
        "object" => v.is_object(),
        "array" => v.is_array(),
        _ => false,
    } {
        Ok(())
    } else {
        Err(Error::invalid())
    }
}
pub(super) fn load_plugins(assets: &Assets) -> Result<Vec<LoadedPlugin>> {
    let root = assets.root.join("plugins");
    if !root.exists() {
        return Ok(Vec::new());
    }
    if fs::symlink_metadata(&root).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(Error::invalid());
    }
    let mut loaded = Vec::new();
    for entry in fs::read_dir(&root).map_err(|_| Error::invalid())? {
        let path = entry.map_err(|_| Error::invalid())?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("py") {
            eprintln!("Python plugins are unsupported and were not loaded");
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::invalid());
        }
        let v: Value = serde_json::from_slice(&fs::read(&path).map_err(|_| Error::invalid())?)
            .map_err(|_| Error::invalid())?;
        domain::validate("PluginManifest", &v)?;
        let m: PluginManifest = serde_json::from_value(v).map_err(|_| Error::invalid())?;
        if path.file_stem().and_then(|s| s.to_str()) != Some(&m.id)
            || m.entry == "."
            || m.entry == ".."
            || !m.entry.ends_with(".rhai")
            || semver::Version::parse(&m.version).is_err()
        {
            return Err(Error::invalid());
        }
        let path = root.join(&m.entry);
        if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(Error::invalid());
        }
        let b = fs::read_to_string(path).map_err(|_| Error::invalid())?;
        if b.len() > 65536 {
            return Err(Error::invalid());
        }
        let ast = rhai::Engine::new()
            .compile(b)
            .map_err(|_| Error::invalid())?;
        loaded.push(LoadedPlugin { manifest: m, ast });
    }
    loaded.sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));
    Ok(loaded)
}
