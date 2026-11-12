use super::*;

pub(super) async fn version(
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<ServerVersion>> {
    i.require(Capability::Read)?;
    Ok(Json(ServerVersion {
        api_version: 2,
        version: env!("CARGO_PKG_VERSION").into(),
    }))
}

pub(super) async fn translations(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<Value>> {
    i.require(Capability::Read)?;
    Ok(Json(
        blocking(a.io.clone(), move || translation_snapshot(a)).await?,
    ))
}

type TranslationCatalog = std::collections::BTreeMap<String, String>;

fn valid_catalog_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn decode_catalog(text: &str) -> Option<TranslationCatalog> {
    serde_json::from_str(text).ok()
}

fn read_catalog(root: &std::path::Path, name: &str) -> Option<TranslationCatalog> {
    if !valid_catalog_name(name) {
        return None;
    }
    let catalog = std::fs::read_to_string(root.join(format!("{name}.json")))
        .ok()
        .and_then(|text| decode_catalog(&text));
    #[cfg(webdeck_embedded_frontend)]
    let catalog =
        catalog.or_else(|| frontend::translation(name).and_then(|text| decode_catalog(&text)));
    catalog
}

fn translation_snapshot(a: App) -> Result<Value> {
    let language = a.config.last_valid().config.settings.language;
    #[cfg(not(webdeck_embedded_frontend))]
    let root = frontend::disk_root("webdeck/translations");
    #[cfg(webdeck_embedded_frontend)]
    let root = std::path::PathBuf::from("webdeck/translations");
    let mut result = read_catalog(&root, "en_US").ok_or_else(Error::execution)?;
    if language != "en_US" {
        if let Some(localized) = read_catalog(&root, &language) {
            result.extend(localized);
        }
    }
    let mut languages = std::collections::BTreeSet::new();
    #[cfg(webdeck_embedded_frontend)]
    languages.extend(
        frontend::EMBEDDED_LANGUAGES
            .iter()
            .map(|name| name.to_string()),
    );
    if let Ok(entries) = std::fs::read_dir(root) {
        languages.extend(
            entries
                .flatten()
                .filter(|entry| {
                    entry.file_type().is_ok_and(|kind| kind.is_file())
                        && entry.path().extension().is_some_and(|ext| ext == "json")
                })
                .filter_map(|entry| {
                    entry
                        .path()
                        .file_stem()
                        .and_then(|name| name.to_str())
                        .filter(|name| valid_catalog_name(name))
                        .map(str::to_owned)
                }),
        );
    } else if languages.is_empty() {
        return Err(Error::execution());
    }
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

pub(super) async fn keyboard_capabilities(
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<Value>> {
    i.local()?;
    i.require(Capability::Input)?;
    let wayland =
        cfg!(target_os = "linux") && std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland");
    Ok(Json(
        json!({"catalog_version":1,"platform": if cfg!(windows) {"windows"} else if wayland {"linux_wayland"} else if cfg!(target_os="linux") {"linux_x11"} else {"unsupported"}, "availability": if wayland {"portal_approval_required"} else {"desktop_access_unverified"}, "fn_status":"unsupported"}),
    ))
}

#[cfg(test)]
mod translation_tests {
    use super::*;

    #[test]
    fn json_catalog_preserves_unicode_equals_and_escaped_lines() {
        let catalog = decode_catalog(r#"{"greeting":"Español = 日本語","help":"line one\nline two","label":"Editar {label}","legacy":"%count%"}"#).unwrap();
        assert_eq!(catalog["greeting"], "Español = 日本語");
        assert_eq!(catalog["help"], "line one\nline two");
        assert_eq!(catalog["label"], "Editar {label}");
        assert_eq!(catalog["legacy"], "%count%");
    }

    #[test]
    fn catalog_requires_a_flat_string_dictionary() {
        assert!(decode_catalog(r#"{"common.settings":42}"#).is_none());
        assert!(decode_catalog(r#"{"common.settings":{"nested":"value"}}"#).is_none());
        assert!(decode_catalog("not json").is_none());
    }

    #[test]
    fn catalog_names_cannot_traverse_the_filesystem() {
        for name in [
            "../en_US",
            "en_US/../../config",
            "/tmp/secret",
            "",
            "en_US.json",
        ] {
            assert!(!valid_catalog_name(name));
        }
        assert!(valid_catalog_name("en_US"));
        assert!(valid_catalog_name("zh_Hant_TW"));
    }
}
