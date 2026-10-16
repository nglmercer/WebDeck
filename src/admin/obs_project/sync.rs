use super::*;
use crate::admin::{provisioning, WebDeckAdminClient};
use std::time::Duration;
pub async fn discover(client: &WebDeckAdminClient) -> Result<Snapshot> {
    let version = client.obs_request("GetVersion", json!({})).await?;
    let collections = client
        .obs_request("GetSceneCollectionList", json!({}))
        .await?;
    let scenes = client.obs_request("GetSceneList", json!({})).await?["scenes"]
        .as_array()
        .cloned()
        .ok_or_else(|| AdminError::new(ErrorKind::Integration, "OBS returned invalid scenes"))?;
    let inputs = client.obs_request("GetInputList", json!({})).await?["inputs"]
        .as_array()
        .cloned()
        .ok_or_else(|| AdminError::new(ErrorKind::Integration, "OBS returned invalid inputs"))?;
    if scenes.len() > 128 || inputs.len() > 256 {
        return Err(AdminError::new(
            ErrorKind::Validation,
            "OBS project discovery exceeds 128 scenes or 256 inputs",
        ));
    }
    let kinds = client.obs_request("GetInputKindList", json!({})).await?["inputKinds"]
        .as_array()
        .map(|v| {
            v.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let mut snapshot = Snapshot {
        version,
        collections,
        scenes,
        inputs,
        kinds,
        ..Default::default()
    };
    for scene in &snapshot.scenes {
        let name = scene["sceneName"]
            .as_str()
            .ok_or_else(|| AdminError::new(ErrorKind::Integration, "OBS scene name missing"))?;
        let items = client
            .obs_request("GetSceneItemList", json!({"sceneName":name}))
            .await?["sceneItems"]
            .as_array()
            .cloned()
            .ok_or_else(|| AdminError::new(ErrorKind::Integration, "OBS scene items missing"))?;
        if items.len() > 256 {
            return Err(AdminError::new(
                ErrorKind::Validation,
                "OBS scene exceeds 256 items",
            ));
        }
        snapshot.items.insert(name.into(), items);
    }
    Ok(snapshot)
}
fn endpoint(client: &WebDeckAdminClient) -> String {
    client.base_url().to_string()
}
pub(super) fn state(project: &Project, client: &WebDeckAdminClient) -> Result<Bindings> {
    let path = project.root.join(".webdeck-state.json");
    let state = if path.exists() {
        serde_json::from_value(read_limit(
            &project.root,
            ".webdeck-state.json",
            16 * 1024 * 1024,
        )?)
        .map_err(|_| {
            invalid(
                ".webdeck-state.json",
                "",
                "invalid_state",
                "Invalid binding state",
            )
        })?
    } else {
        Bindings {
            project: project.manifest.project_id.clone(),
            collection: project.manifest.collection.clone(),
            endpoint: endpoint(client),
            ..Default::default()
        }
    };
    if state.project != project.manifest.project_id
        || state.collection != project.manifest.collection
        || state.endpoint != endpoint(client)
    {
        return Err(invalid(
            ".webdeck-state.json",
            "",
            "state_scope_conflict",
            "State belongs to another project, WebDeck endpoint or collection",
        ));
    }
    Ok(state)
}
fn save(project: &Project, state: &Bindings) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(state)
        .map_err(|_| AdminError::new(ErrorKind::Generic, "Cannot encode bindings"))?;
    crate::storage::atomic_replace(&project.root.join(".webdeck-state.json"), &bytes)
        .map_err(AdminError::from_domain)
}
fn bind(project: &Project, snapshot: &Snapshot, state: &mut Bindings) {
    for scene in &project.scenes {
        if let Some(found) = snapshot
            .scenes
            .iter()
            .find(|s| s["sceneName"] == scene.name)
        {
            state.scenes.insert(
                scene.id.clone(),
                Binding {
                    name: scene.name.clone(),
                    uuid: found["sceneUuid"].as_str().map(String::from),
                },
            );
        }
        for item in &scene.items {
            let source = item
                .input_name
                .as_deref()
                .or_else(|| {
                    item.scene_id.as_deref().and_then(|id| {
                        project
                            .scenes
                            .iter()
                            .find(|s| s.id == id)
                            .map(|s| s.name.as_str())
                    })
                })
                .expect("validated");
            if item.kind == "input" {
                if let Some(found) = snapshot.inputs.iter().find(|s| s["inputName"] == source) {
                    state.inputs.insert(
                        item.input_id.as_deref().unwrap_or(source).into(),
                        Binding {
                            name: source.into(),
                            uuid: found["inputUuid"].as_str().map(String::from),
                        },
                    );
                }
            }
            if let Some(found) = snapshot.items.get(&scene.name).and_then(|items| {
                let matching = items
                    .iter()
                    .filter(|i| i["sourceName"] == source)
                    .collect::<Vec<_>>();
                if matching.len() == 1 {
                    Some(matching[0])
                } else {
                    None
                }
            }) {
                if let Some(id) = found["sceneItemId"].as_i64() {
                    let resource = format!("item/{}/{}", scene.id, item.id);
                    state
                        .item_sources
                        .insert(resource.clone(), source.to_string());
                    state.items.insert(resource, id);
                }
            }
        }
    }
}
pub async fn reconcile(
    client: &WebDeckAdminClient,
    path: &Path,
    dry_run: bool,
    adopt: bool,
    revision: Option<u64>,
) -> Result<Outcome> {
    reconcile_with_prune(client, path, dry_run, adopt, revision, false).await
}
pub async fn reconcile_with_prune(
    client: &WebDeckAdminClient,
    path: &Path,
    dry_run: bool,
    adopt: bool,
    revision: Option<u64>,
    prune: bool,
) -> Result<Outcome> {
    let started = std::time::Instant::now();
    let project = load(path)?;
    let _lock = if dry_run {
        None
    } else {
        Some(project_lock(&project)?)
    };
    let mut state = state(&project, client)?;
    let mut snapshot = discover(client).await?;
    let connection = snapshot.version["webdeckConnection"]
        .as_str()
        .unwrap_or("")
        .to_string();
    if !state.connection.is_empty() && state.connection != connection {
        return Err(invalid(
            ".webdeck-state.json",
            "",
            "connection_changed",
            "Configured OBS endpoint changed; inspect bindings before adopting another host",
        ));
    }
    state.connection = connection.clone();
    let pinned = client.pin_obs_connection(&connection);
    let client = &pinned;
    if snapshot.collections["currentSceneCollectionName"].as_str()
        != Some(&project.manifest.collection)
    {
        snapshot.scenes.clear();
        snapshot.inputs.clear();
        snapshot.items.clear();
        snapshot.settings.clear();
    }
    // Only fetch settings of declared inputs; mutable settings are never cached across runs.
    for scene in &project.scenes {
        for item in &scene.items {
            if let Some(name) = &item.input_name {
                if item.settings.is_some()
                    && snapshot.inputs.iter().any(|i| i["inputName"] == *name)
                {
                    snapshot.settings.insert(
                        name.clone(),
                        client
                            .obs_request("GetInputSettings", json!({"inputName":name}))
                            .await?["inputSettings"]
                            .clone(),
                    );
                }
            }
        }
    }
    let mut plan = plan(&project, &snapshot, &state, adopt);
    if prune {
        super::prune::plan_prune(&project, &snapshot, &state, &mut plan);
    }
    if dry_run {
        return Ok(Outcome::ok(
            json!({"state":"planned","dry_run":true,"plan":plan,"webdeck":button_preview(client,&project,&snapshot,&state).await?}),
            "OBS project plan (no writes).",
        ));
    }
    if !plan.conflicts.is_empty() {
        return Err(AdminError::with_details(
            ErrorKind::Validation,
            "OBS project has conflicts; inspect diff before applying",
            json!({"plan":plan}),
        ));
    }
    let config = client.get_config().await?;
    client.ensure_revision(revision, config.revision)?;
    state.journal.clear();
    state.partial = true;
    // Persist a recovery marker before sending any mutations. No write occurs in dry-run.
    bind(&project, &snapshot, &mut state);
    save(&project, &state)?;
    for operation in plan.operations.iter().filter(|op| op.kind != "noop") {
        if started.elapsed() > Duration::from_secs(120) {
            save(&project, &state)?;
            return partial(&plan,&state,"Project apply exceeded its two-minute budget. Re-run diff and safely retry remaining operations.");
        }
        eprintln!("{} {} {}", operation.id, operation.kind, operation.resource);
        let result = apply_operation(client, operation, &project.manifest.collection).await;
        match result {
            Ok(response) => {
                state.journal.push(json!({"operation":operation.id,"resource":operation.resource,"state":"applied","before":operation.before}));
                if operation.request == "CreateScene" {
                    let id = operation.resource.trim_start_matches("scene/");
                    state.scenes.insert(
                        id.into(),
                        Binding {
                            name: operation.data["sceneName"].as_str().unwrap_or("").into(),
                            uuid: response["sceneUuid"].as_str().map(String::from),
                        },
                    );
                }
                if operation.request == "SetSceneName" {
                    if let Some(binding) = state
                        .scenes
                        .get_mut(operation.resource.trim_start_matches("scene/"))
                    {
                        binding.name = operation.data["newSceneName"].as_str().unwrap_or("").into();
                    }
                }
                if operation.request == "CreateInput" {
                    let name = operation.data["inputName"].as_str().unwrap_or("");
                    state.inputs.insert(
                        operation.resource.trim_start_matches("input/").into(),
                        Binding {
                            name: name.into(),
                            uuid: response["inputUuid"].as_str().map(String::from),
                        },
                    );
                }
                if operation.request == "SetInputName" {
                    if let Some(binding) = state
                        .inputs
                        .get_mut(operation.resource.trim_start_matches("input/"))
                    {
                        binding.name = operation.data["newInputName"].as_str().unwrap_or("").into();
                    }
                }
                if operation.request == "CreateSceneItem" {
                    if let Some(id) = response["sceneItemId"].as_i64() {
                        state.items.insert(operation.resource.clone(), id);
                        if let Some(source) = operation.data["sourceName"].as_str() {
                            state
                                .item_sources
                                .insert(operation.resource.clone(), source.into());
                        }
                    }
                }
                if operation.kind == "delete" {
                    if operation.request == "RemoveScene" {
                        state
                            .scenes
                            .remove(operation.resource.trim_start_matches("scene/"));
                    }
                    if operation.request == "RemoveInput" {
                        state
                            .inputs
                            .remove(operation.resource.trim_start_matches("input/"));
                    }
                    if operation.request == "RemoveSceneItem" {
                        state.items.remove(&operation.resource);
                        state.item_sources.remove(&operation.resource);
                    }
                }
                if save(&project, &state).is_err() {
                    return partial(&plan,&state,"OBS changed but binding state could not be saved. Inspect OBS and use explicit adoption before retrying.");
                }
            }
            Err(error) => {
                state
                    .journal
                    .push(json!({"operation":operation.id,"state":"failed_or_unknown","error":error.info()}));
                save(&project, &state)?;
                return partial(&plan,&state,"OBS operation failed; inspect OBS, then diff and retry. Live changes were not rolled back.");
            }
        }
    }
    let mut final_snapshot = match discover(client).await {Ok(actual)=>actual,Err(_)=>return partial(&plan,&state,"Final OBS discovery failed. Inspect OBS and retry diff; live changes may have applied.")};
    for operation in plan.operations.iter().filter(|op| op.kind == "delete") {
        let remains = match operation.request.as_str() {
            "RemoveScene" => final_snapshot
                .scenes
                .iter()
                .any(|value| value["sceneUuid"] == operation.before["uuid"]),
            "RemoveInput" => final_snapshot
                .inputs
                .iter()
                .any(|value| value["inputUuid"] == operation.before["uuid"]),
            "RemoveSceneItem" => final_snapshot
                .items
                .get(operation.data["sceneName"].as_str().unwrap_or(""))
                .is_some_and(|items| {
                    items
                        .iter()
                        .any(|item| item["sceneItemId"] == operation.data["sceneItemId"])
                }),
            _ => false,
        };
        if remains {
            return partial(
                &plan,
                &state,
                "Deleted resource still exists during verification; inspect OBS and journal",
            );
        }
    }
    bind(&project, &final_snapshot, &mut state);
    for scene in &project.scenes {
        for item in &scene.items {
            if let Some(name) = &item.input_name {
                if item.settings.is_some() {
                    match client
                        .obs_request("GetInputSettings", json!({"inputName":name}))
                        .await
                    {
                        Ok(value) => {
                            final_snapshot
                                .settings
                                .insert(name.clone(), value["inputSettings"].clone());
                        }
                        Err(_) => {
                            return partial(
                                &plan,
                                &state,
                                "Input settings verification failed; inspect OBS and retry diff",
                            )
                        }
                    }
                }
            }
        }
    }
    let verification = super::planner::plan(&project, &final_snapshot, &state, false);
    if !verification.conflicts.is_empty()
        || verification.operations.iter().any(|op| op.kind != "noop")
    {
        save(&project, &state)?;
        return partial(
            &plan,
            &state,
            "Final verification found differences. Inspect OBS and rerun diff before retrying.",
        );
    }
    if let Some(deck) = &project.manifest.webdeck {
        if deck.generate_buttons {
            let result = if deck.sync_managed_buttons {
                provisioning::sync_obs(client, &deck.folder_id, Some(config.revision), false, false)
                    .await
            } else {
                client
                    .obs()
                    .ensure_buttons(&deck.folder_id, Some(config.revision), false)
                    .await
            };
            if result.is_err() {
                save(&project, &state)?;
                return partial(&plan,&state,"OBS succeeded but WebDeck configuration failed or its revision changed. Retry with the current revision; deck changes were not overwritten.");
            }
        }
    }
    state.partial = false;
    save(&project, &state)?;
    let changed = plan.operations.iter().any(|op| op.kind != "noop");
    Ok(Outcome::ok(
        json!({"state":if changed {"applied"}else{"noop"},"plan":plan,"journal":state.journal}),
        "OBS project verified.",
    ))
}
fn partial(plan: &Plan, state: &Bindings, message: &str) -> Result<Outcome> {
    Ok(Outcome::failed(
        json!({"state":"partial_failure","plan":plan,"journal":state.journal,"recovery":message}),
        crate::admin::error::ErrorInfo {
            code: ErrorKind::Integration.code(),
            message: message.into(),
            details: None,
        },
        crate::admin::ExitCode::Integration as i32,
        message,
    ))
}
pub(super) async fn apply_operation(
    client: &WebDeckAdminClient,
    operation: &Operation,
    collection: &str,
) -> Result<Value> {
    if operation.request != "CreateSceneCollection"
        && client
            .obs_request("GetSceneCollectionList", json!({}))
            .await?["currentSceneCollectionName"]
            != collection
    {
        return Err(AdminError::new(
            ErrorKind::Validation,
            "OBS collection changed during apply; stopped before mutation",
        ));
    }
    if operation.kind == "delete" {
        if operation.request == "RemoveSceneItem" {
            let items = client
                .obs_request(
                    "GetSceneItemList",
                    json!({"sceneName":operation.data["sceneName"]}),
                )
                .await?;
            if !items["sceneItems"].as_array().is_some_and(|items| {
                items.iter().any(|item| {
                    item["sceneItemId"] == operation.data["sceneItemId"]
                        && item["sourceName"] == operation.before["sourceName"]
                })
            }) {
                return Err(AdminError::new(
                    ErrorKind::Validation,
                    "Scene item changed since prune planning",
                ));
            }
        } else {
            let scene = operation.request == "RemoveScene";
            let values = client
                .obs_request(
                    if scene {
                        "GetSceneList"
                    } else {
                        "GetInputList"
                    },
                    json!({}),
                )
                .await?;
            let (array, name, uuid) = if scene {
                ("scenes", "sceneName", "sceneUuid")
            } else {
                ("inputs", "inputName", "inputUuid")
            };
            if !values[array].as_array().is_some_and(|items| {
                items.iter().any(|item| {
                    item[name] == operation.before["name"] && item[uuid] == operation.before["uuid"]
                })
            }) {
                return Err(AdminError::new(
                    ErrorKind::Validation,
                    "Resource UUID changed since prune planning",
                ));
            }
            let all = client.obs_request("GetSceneList", json!({})).await?;
            if let Some(scenes) = all["scenes"].as_array() {
                for parent in scenes {
                    let items = client
                        .obs_request("GetSceneItemList", json!({"sceneName":parent["sceneName"]}))
                        .await?;
                    if items["sceneItems"].as_array().is_some_and(|items| {
                        items
                            .iter()
                            .any(|item| item["sourceName"] == operation.before["name"])
                            || (scene
                                && parent["sceneName"] == operation.before["name"]
                                && !items.is_empty())
                    }) {
                        return Err(AdminError::new(
                            ErrorKind::Validation,
                            "Preserved/new items prevent resource deletion",
                        ));
                    }
                }
            }
        }
    }
    let mut data = operation.data.clone();
    if operation.request == "CreateSceneItem"
        || (data.get("sceneItemId").is_some() && operation.kind != "delete")
    {
        let items = client
            .obs_request("GetSceneItemList", json!({"sceneName":data["sceneName"]}))
            .await?;
        let matching = items["sceneItems"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter(|i| i["sourceName"] == data["sourceName"])
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if matching.len() > 1 {
            return Err(AdminError::new(
                ErrorKind::Validation,
                "Scene item became ambiguous during apply",
            ));
        }
        if operation.request == "CreateSceneItem" {
            if let Some(item) = matching.first() {
                return Ok(json!({"sceneItemId":item["sceneItemId"]}));
            }
        } else {
            data["sceneItemId"] = matching
                .first()
                .map(|i| i["sceneItemId"].clone())
                .ok_or_else(|| {
                    AdminError::new(ErrorKind::Integration, "Scene item missing during apply")
                })?;
            data.as_object_mut().expect("object").remove("sourceName");
        }
    }
    // Allow the OBS collection switch to settle through an explicit read, never blind replay.
    let result = client
        .obs_request_authorized(&operation.request, data, operation.kind == "delete")
        .await?;
    if operation.request == "CreateSceneCollection" {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Ok(result)
}

pub(super) async fn managed_snapshot(
    client: &WebDeckAdminClient,
    project: &Project,
) -> Result<(Snapshot, Bindings)> {
    let state = state(project, client)?;
    let snapshot = discover(client).await?;
    if !state.connection.is_empty() && snapshot.version["webdeckConnection"] != state.connection {
        return Err(invalid(
            ".webdeck-state.json",
            "",
            "connection_changed",
            "OBS endpoint changed",
        ));
    }
    Ok((snapshot, state))
}

async fn button_preview(
    client: &WebDeckAdminClient,
    project: &Project,
    snapshot: &Snapshot,
    state: &Bindings,
) -> Result<Value> {
    let Some(deck) = &project.manifest.webdeck else {
        return Ok(Value::Null);
    };
    if !deck.generate_buttons {
        return Ok(Value::Null);
    }
    let current = client.get_config().await?;
    let catalog = client.catalog().await?;
    let recipe = catalog
        .automation
        .as_ref()
        .and_then(|a| {
            a.button_recipes
                .iter()
                .find(|recipe| recipe.id == "obs-scenes")
        })
        .ok_or_else(|| AdminError::new(ErrorKind::NotFound, "OBS scene recipe missing"))?;
    let mut scenes = snapshot.scenes.clone();
    for scene in &project.scenes {
        let uuid = state
            .scenes
            .get(&scene.id)
            .and_then(|binding| binding.uuid.as_deref());
        if let Some(actual) = scenes.iter_mut().find(|actual| {
            uuid.is_some_and(|uuid| actual["sceneUuid"] == uuid)
                || actual["sceneName"] == scene.name
        }) {
            actual["sceneName"] = json!(scene.name);
        } else {
            scenes.push(json!({"sceneName":scene.name}));
        }
    }
    let result = json!({"scenes":scenes});
    let (config, changes) = if deck.sync_managed_buttons {
        provisioning::plan_obs_sync(
            &current.config,
            &catalog,
            recipe,
            &result,
            &deck.folder_id,
            false,
        )?
    } else {
        provisioning::plan(&current.config, &catalog, recipe, &result, &deck.folder_id)?
    };
    Ok(
        json!({"revision":current.revision,"folder":deck.folder_id,"changed":config!=current.config,"changes":changes,"note":"New OBS UUIDs are assigned during apply; generated button recipe IDs use scene names"}),
    )
}

pub(super) fn project_lock(project: &Project) -> Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let file = options
        .open(project.root.join(".webdeck-state.lock"))
        .map_err(|_| {
            invalid(
                ".webdeck-state.lock",
                "",
                "lock_failed",
                "Cannot open project lock",
            )
        })?;
    file.try_lock().map_err(|_| {
        invalid(
            ".webdeck-state.lock",
            "",
            "project_busy",
            "Another project apply is running",
        )
    })?;
    Ok(file)
}
