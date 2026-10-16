//! Managed-only pruning. UUID ownership and inbound references are checked before deletion.
use super::*;
pub fn plan_prune(project: &Project, snapshot: &Snapshot, state: &Bindings, plan: &mut Plan) {
    if !project
        .manifest
        .sync
        .as_ref()
        .is_some_and(|sync| sync.mode == "managed")
    {
        plan.conflicts.push(json!({"code":"prune_requires_managed","message":"Use sync.mode managed and explicit --prune-managed --allow-destructive"}));
        return;
    }
    let desired_scenes = project
        .scenes
        .iter()
        .map(|s| s.id.as_str())
        .collect::<HashSet<_>>();
    let desired_items = project
        .scenes
        .iter()
        .flat_map(|s| {
            s.items
                .iter()
                .map(move |i| format!("item/{}/{}", s.id, i.id))
        })
        .collect::<HashSet<_>>();
    let desired_inputs = project
        .scenes
        .iter()
        .flat_map(|s| s.items.iter())
        .filter(|i| i.kind == "input")
        .filter_map(|i| i.input_id.as_deref().or(i.input_name.as_deref()))
        .collect::<HashSet<_>>();
    let mut stale_scenes = BTreeMap::new();
    for (id, binding) in &state.scenes {
        if !desired_scenes.contains(id.as_str()) {
            if let Some(found) = binding.uuid.as_deref().and_then(|uuid| {
                snapshot
                    .scenes
                    .iter()
                    .find(|scene| scene["sceneUuid"] == uuid)
            }) {
                if let Some(name) = found["sceneName"].as_str() {
                    stale_scenes.insert(id.clone(), name.to_string());
                }
            } else if snapshot
                .scenes
                .iter()
                .any(|scene| scene["sceneName"] == binding.name)
            {
                plan.conflicts.push(json!({"code":"prune_identity_unverified","resource":id,"message":"Cannot prune without a live UUID binding"}));
            }
        }
    }
    let stale_names = stale_scenes
        .values()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let mut operations = Vec::new();
    let mut removed_items = HashSet::new();
    for (resource, id) in &state.items {
        if !desired_items.contains(resource) {
            let scene_id = resource.split('/').nth(1).unwrap_or("");
            let Some(binding) = state.scenes.get(scene_id) else {
                continue;
            };
            let Some(scene) = binding.uuid.as_deref().and_then(|uuid| {
                snapshot
                    .scenes
                    .iter()
                    .find(|scene| scene["sceneUuid"] == uuid)
            }) else {
                continue;
            };
            let name = scene["sceneName"].as_str().unwrap_or("");
            let Some(item) = snapshot.items.get(name).and_then(|items| {
                items
                    .iter()
                    .find(|item| item["sceneItemId"].as_i64() == Some(*id))
            }) else {
                continue;
            };
            let source = item["sourceName"].as_str().unwrap_or("");
            if state
                .item_sources
                .get(resource)
                .is_none_or(|owned| owned != source)
            {
                plan.conflicts
                    .push(json!({"code":"prune_item_identity_unverified","resource":resource}));
                continue;
            }
            removed_items.insert((name.to_string(), *id));
            {
                operations.push((
                    resource.clone(),
                    "RemoveSceneItem",
                    json!({"sceneName":name,"sceneItemId":id}),
                    item.clone(),
                ));
            }
        }
    }
    for (id, name) in &stale_scenes {
        if snapshot.items.get(name).is_some_and(|items| {
            items.iter().any(|item| {
                !removed_items.contains(&(name.clone(), item["sceneItemId"].as_i64().unwrap_or(-1)))
            })
        }) {
            plan.conflicts.push(json!({"code":"managed_scene_has_unmanaged_items","resource":id,"message":"Preserve or explicitly manage the remaining scene items before pruning this scene"}));
        }
    }
    for (id, name) in &stale_scenes {
        let referenced = snapshot.items.iter().any(|(parent, items)| {
            !stale_names.contains(parent.as_str())
                && items.iter().any(|item| {
                    item["sourceName"] == *name
                        && !removed_items
                            .contains(&(parent.clone(), item["sceneItemId"].as_i64().unwrap_or(-1)))
                })
        });
        if referenced {
            plan.conflicts.push(json!({"code":"managed_scene_still_referenced","resource":id,"message":"A preserved scene item references this scene; remove that reference explicitly"}));
        } else {
            operations.push((
                format!("scene/{id}"),
                "RemoveScene",
                json!({"sceneName":name}),
                json!({"name":name,"uuid":state.scenes[id].uuid}),
            ));
        }
    }
    for (id, binding) in &state.inputs {
        if !desired_inputs.contains(id.as_str()) {
            let Some(input) = binding.uuid.as_deref().and_then(|uuid| {
                snapshot
                    .inputs
                    .iter()
                    .find(|input| input["inputUuid"] == uuid)
            }) else {
                if snapshot
                    .inputs
                    .iter()
                    .any(|input| input["inputName"] == binding.name)
                {
                    plan.conflicts
                        .push(json!({"code":"prune_identity_unverified","resource":id}));
                }
                continue;
            };
            let name = input["inputName"].as_str().unwrap_or("");
            let referenced = snapshot.items.iter().any(|(parent, items)| {
                !stale_names.contains(parent.as_str())
                    && items.iter().any(|item| {
                        item["sourceName"] == name
                            && !removed_items.contains(&(
                                parent.clone(),
                                item["sceneItemId"].as_i64().unwrap_or(-1),
                            ))
                    })
            });
            if referenced {
                plan.conflicts
                    .push(json!({"code":"managed_input_still_referenced","resource":id}));
            } else {
                operations.push((
                    format!("input/{id}"),
                    "RemoveInput",
                    json!({"inputName":name}),
                    json!({"name":name,"uuid":binding.uuid}),
                ));
            }
        }
    }
    for (resource, request, data, before) in operations {
        if !snapshot.version["availableRequests"]
            .as_array()
            .is_some_and(|list| list.iter().any(|v| v == request))
        {
            plan.conflicts
                .push(json!({"code":"unsupported_request","request":request}));
        }
        let id = format!("op-{:04}", plan.operations.len() + 1);
        plan.operations.push(Operation {
            id,
            kind: "delete".into(),
            resource,
            request: request.into(),
            data,
            before,
            after: Value::Null,
        });
    }
}
