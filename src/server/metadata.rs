use super::*;

pub(super) async fn translations(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<Value>> {
    i.require(Capability::Read)?;
    Ok(Json(
        blocking(a.io.clone(), move || translation_snapshot(a)).await?,
    ))
}

fn translation_snapshot(a: App) -> Result<Value> {
    let language = a.config.last_valid().config.settings.language;
    let root = std::path::Path::new("webdeck/translations");
    let mut result = std::collections::BTreeMap::new();
    for name in ["en_US", language.as_str()] {
        if let Ok(text) = std::fs::read_to_string(root.join(format!("{name}.lang"))) {
            for line in text.lines() {
                if line.starts_with('#') || line.starts_with("//") {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    result.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
        }
    }
    let languages = std::fs::read_dir(root)
        .map_err(|_| Error::execution())?
        .flatten()
        .filter(|e| {
            e.file_type().is_ok_and(|t| t.is_file())
                && e.path().extension().is_some_and(|ext| ext == "lang")
        })
        .filter_map(|e| {
            e.path()
                .file_stem()
                .and_then(|s| s.to_str())
                .map(str::to_owned)
        })
        .collect::<Vec<_>>();
    Ok(json!({"api_version":2,"translations":result,"languages":languages}))
}

pub(super) async fn audio_devices(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<AudioDevices>> {
    i.require(Capability::Settings)?;
    let d = blocking(a.queries.clone(), || {
        use rodio::cpal::traits::{DeviceTrait, HostTrait};
        let h = rodio::cpal::default_host();
        let inputs = h
            .input_devices()
            .map_err(|_| Error::execution())?
            .filter_map(|d| d.description().ok().map(|n| n.name().to_string()))
            .collect();
        let outputs = h
            .output_devices()
            .map_err(|_| Error::execution())?
            .filter_map(|d| d.description().ok().map(|n| n.name().to_string()))
            .collect();
        Ok::<_, Error>(AudioDevices {
            api_version: 2,
            inputs,
            outputs,
        })
    })
    .await?;
    Ok(Json(d))
}

pub(super) async fn boot(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<DeckBoot>> {
    i.require(Capability::Read)?;
    Ok(Json(
        blocking(a.io.clone(), move || boot_snapshot(a, i)).await?,
    ))
}

fn boot_snapshot(a: App, i: Identity) -> Result<DeckBoot> {
    let s = a.config.snapshot()?;
    let mut layout = s.config.layout;
    layout.extensions = presentation(&layout.extensions);
    let mut button_capabilities = std::collections::BTreeMap::new();
    for f in &mut layout.folders {
        f.extensions.clear();
        for b in &mut f.buttons {
            b.extensions = presentation(&b.extensions);
            if let Some(command) = domain::action_command(&b.action) {
                let r = resolve_request(
                    &a,
                    CommandRequest {
                        request_id: "boot".into(),
                        command,
                    },
                )?;
                button_capabilities.insert(b.id.clone(), r.command.capability());
                b.action = ButtonAction::Command {
                    command: Command::Button {
                        button_id: b.id.clone(),
                    },
                };
            }
        }
    }
    Ok(DeckBoot {
        api_version: 2,
        revision: s.revision,
        layout,
        language: s.config.settings.language,
        can_edit: i.local && i.capabilities.contains(&Capability::Settings),
        capabilities: i.capabilities,
        button_capabilities,
    })
}
pub(super) fn presentation(
    extensions: &std::collections::BTreeMap<String, Value>,
) -> std::collections::BTreeMap<String, Value> {
    let mut result = std::collections::BTreeMap::new();
    if let Some(appearance) = extensions.get("appearance").and_then(Value::as_object) {
        let clean: serde_json::Map<String, Value> = appearance
            .iter()
            .filter(|(key, value)| {
                matches!(
                    key.as_str(),
                    "gap" | "button_height" | "radius" | "icon_size" | "columns" | "rows" | "cell"
                ) && value.is_number()
                    || matches!(key.as_str(), "show_labels" | "show_label") && value.is_boolean()
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        result.insert("appearance".into(), Value::Object(clean));
    }
    result
}
