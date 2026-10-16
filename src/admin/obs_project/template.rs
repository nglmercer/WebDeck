//! Explicit property propagation for managed nested-scene instances only.
use super::*;
use crate::admin::WebDeckAdminClient;
pub async fn sync_instances(
    client: &WebDeckAdminClient,
    template: &Path,
    dry_run: bool,
) -> Result<Outcome> {
    let template = template
        .canonicalize()
        .map_err(|_| invalid("template", "", "missing_file", "Template file not found"))?;
    let root = template
        .ancestors()
        .skip(1)
        .take(4)
        .find(|parent| parent.join("project.json").is_file())
        .ok_or_else(|| {
            invalid(
                "template",
                "",
                "missing_project",
                "Template must belong to an OBS project",
            )
        })?;
    let project = load(root)?;
    let relative = template
        .strip_prefix(&project.root)
        .map_err(|_| {
            invalid(
                "template",
                "",
                "path_escape",
                "Template must stay inside the project",
            )
        })?
        .to_string_lossy()
        .replace('\\', "/");
    if !project
        .manifest
        .shared
        .iter()
        .chain(&project.manifest.scenes)
        .any(|file| file == &relative)
    {
        return Err(invalid(
            &relative,
            "",
            "not_included",
            "Template must be included in project.json",
        ));
    }
    let value = read(&project.root, &relative)?;
    let scene: Scene = serde_json::from_value(value).map_err(|_| {
        invalid(
            &relative,
            "",
            "invalid_scene",
            "Template must be a scene file",
        )
    })?;
    let properties = scene.instance_properties.ok_or_else(|| {
        invalid(
            &relative,
            "/instanceProperties",
            "missing_properties",
            "Declare enabled, transform or index in instanceProperties to propagate them",
        )
    })?;
    let _lock = if dry_run {
        None
    } else {
        Some(super::sync::project_lock(&project)?)
    };
    let (snapshot, state) = super::sync::managed_snapshot(client, &project).await?;
    if snapshot.collections["currentSceneCollectionName"] != project.manifest.collection {
        return Err(invalid(
            "project.json",
            "/collection",
            "collection_not_selected",
            "Select the project collection before syncing properties",
        ));
    }
    let pinned =
        client.pin_obs_connection(snapshot.version["webdeckConnection"].as_str().unwrap_or(""));
    let client = &pinned;
    let plan = plan_instances(&project, &snapshot, &state, &scene.id, &properties);
    if dry_run {
        return Ok(Outcome::ok(
            json!({"state":"planned","dry_run":true,"plan":plan}),
            "Managed instance property plan (no writes).",
        ));
    }
    if !plan.conflicts.is_empty() {
        return Err(AdminError::with_details(
            ErrorKind::Validation,
            "Managed instance template has conflicts",
            json!({"plan":plan}),
        ));
    }
    let mut journal = Vec::new();
    let persist = |journal: &Vec<Value>, partial: bool| -> Result<()> {
        let value = json!({"project":project.manifest.project_id,"collection":project.manifest.collection,"partial":partial,"plan":plan,"journal":journal});
        crate::storage::atomic_replace(
            &project.root.join(".webdeck-instance-journal.json"),
            &serde_json::to_vec_pretty(&value).map_err(|_| {
                AdminError::new(ErrorKind::Generic, "Cannot encode instance journal")
            })?,
        )
        .map_err(AdminError::from_domain)
    };
    persist(&journal, true)?;
    let started = std::time::Instant::now();
    for operation in plan.operations.iter().filter(|op| op.kind != "noop") {
        if started.elapsed() > std::time::Duration::from_secs(120) {
            return Err(AdminError::with_details(
                ErrorKind::Integration,
                "Template apply budget exhausted; inspect journal and retry diff",
                json!({"state":"partial_failure","journal":journal}),
            ));
        }
        if super::sync::apply_operation(client, operation, &project.manifest.collection)
            .await
            .is_err()
        {
            return Ok(Outcome::failed(
                json!({"state":"partial_failure","plan":plan,"journal":journal,"recovery":"Inspect OBS and rerun template diff; no rollback was attempted"}),
                crate::admin::error::ErrorInfo {
                    code: ErrorKind::Integration.code(),
                    message: "Instance property sync partially applied".into(),
                    details: None,
                },
                crate::admin::ExitCode::Integration as i32,
                "Instance property sync partially applied",
            ));
        }
        journal.push(json!({"id":operation.id,"before":operation.before,"state":"applied"}));
        persist(&journal, true)?;
    }
    let verified = super::sync::discover(client).await?;
    let remaining = plan_instances(&project, &verified, &state, &scene.id, &properties);
    if !remaining.conflicts.is_empty() || remaining.operations.iter().any(|op| op.kind != "noop") {
        return Err(AdminError::with_details(
            ErrorKind::Integration,
            "Instance property verification failed; inspect OBS",
            json!({"state":"partial_failure","journal":journal}),
        ));
    }
    persist(&journal, false)?;
    Ok(Outcome::ok(
        json!({"state":if journal.is_empty(){"noop"}else{"applied"},"plan":plan,"journal":journal}),
        "Managed instance properties verified.",
    ))
}
pub fn plan_instances(
    project: &Project,
    snapshot: &Snapshot,
    state: &Bindings,
    scene_id: &str,
    properties: &InstanceProperties,
) -> Plan {
    let mut plan = Plan {
        project: project.manifest.project_id.clone(),
        collection: project.manifest.collection.clone(),
        operations: vec![],
        conflicts: vec![],
        warnings: vec![],
    };
    for parent in &project.scenes {
        for item in &parent.items {
            if item.scene_id.as_deref() == Some(scene_id) {
                let resource = format!("item/{}/{}", parent.id, item.id);
                let actual = state.items.get(&resource).and_then(|id| {
                    snapshot.items.get(&parent.name).and_then(|items| {
                        items
                            .iter()
                            .find(|value| value["sceneItemId"].as_i64() == Some(*id))
                    })
                });
                let Some(actual) = actual else {
                    plan.conflicts
                        .push(json!({"code":"unmanaged_or_missing_instance","resource":resource}));
                    continue;
                };
                if state
                    .item_sources
                    .get(&resource)
                    .is_none_or(|source| actual["sourceName"] != *source)
                {
                    plan.conflicts
                        .push(json!({"code":"instance_identity_changed","resource":resource}));
                    continue;
                }
                for (request, field, desired, override_value) in [
                    (
                        "SetSceneItemEnabled",
                        "sceneItemEnabled",
                        properties.enabled.map(Value::Bool),
                        item.enabled.map(Value::Bool),
                    ),
                    (
                        "SetSceneItemTransform",
                        "sceneItemTransform",
                        properties.transform.clone(),
                        item.transform.clone(),
                    ),
                    (
                        "SetSceneItemIndex",
                        "sceneItemIndex",
                        properties.index.map(|v| json!(v)),
                        item.index.map(|v| json!(v)),
                    ),
                ] {
                    let Some(mut desired) = desired else { continue };
                    // Explicit per-instance values win; partial transform overrides merge by field.
                    if let Some(override_value) = override_value {
                        if let (Some(base), Some(overrides)) =
                            (desired.as_object_mut(), override_value.as_object())
                        {
                            base.extend(overrides.clone());
                        } else {
                            desired = override_value;
                        }
                    }
                    let before = actual[field].clone();
                    let changed = if let Some(map) = desired.as_object() {
                        map.iter().any(|(key, value)| before[key] != *value)
                    } else {
                        before != desired
                    };
                    let mut data = json!({"sceneName":parent.name,"sceneItemId":actual["sceneItemId"],"sourceName":actual["sourceName"]});
                    data[field] = desired.clone();
                    if changed
                        && !snapshot.version["availableRequests"]
                            .as_array()
                            .is_some_and(|requests| requests.iter().any(|value| value == request))
                    {
                        plan.conflicts
                            .push(json!({"code":"unsupported_request","request":request}));
                    }
                    let id = format!("op-{:04}", plan.operations.len() + 1);
                    plan.operations.push(Operation {
                        id,
                        kind: if changed { "update" } else { "noop" }.into(),
                        resource: resource.clone(),
                        request: request.into(),
                        data,
                        before,
                        after: desired,
                    });
                }
            }
        }
    }
    plan
}
