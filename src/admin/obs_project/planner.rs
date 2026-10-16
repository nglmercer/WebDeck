use super::*;
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bindings {
    pub project: String,
    pub endpoint: String,
    pub collection: String,
    #[serde(default)]
    pub connection: String,
    #[serde(default)]
    pub scenes: BTreeMap<String, Binding>,
    #[serde(default)]
    pub inputs: BTreeMap<String, Binding>,
    #[serde(default)]
    pub items: BTreeMap<String, i64>,
    #[serde(default)]
    pub item_sources: BTreeMap<String, String>,
    #[serde(default)]
    pub journal: Vec<Value>,
    #[serde(default)]
    pub partial: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub name: String,
    pub uuid: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: Value,
    pub collections: Value,
    pub scenes: Vec<Value>,
    pub inputs: Vec<Value>,
    pub items: BTreeMap<String, Vec<Value>>,
    pub settings: BTreeMap<String, Value>,
    pub kinds: Vec<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Operation {
    pub id: String,
    pub kind: String,
    pub resource: String,
    pub request: String,
    pub data: Value,
    pub before: Value,
    pub after: Value,
}
#[derive(Debug, Clone, Serialize)]
pub struct Plan {
    pub project: String,
    pub collection: String,
    pub operations: Vec<Operation>,
    pub conflicts: Vec<Value>,
    pub warnings: Vec<String>,
}
fn patch_diff(desired: &Value, actual: &Value) -> bool {
    if let Some(map) = desired.as_object() {
        map.iter()
            .any(|(key, value)| patch_diff(value, &actual[key]))
    } else {
        desired != actual
    }
}
fn resolve<'a>(
    values: &'a [Value],
    binding: Option<&Binding>,
    name: &str,
    name_field: &str,
    uuid_field: &str,
) -> Option<&'a Value> {
    if let Some(uuid) = binding.and_then(|b| b.uuid.as_ref()) {
        return values.iter().find(|v| v[uuid_field].as_str() == Some(uuid));
    }
    values
        .iter()
        .find(|v| v[name_field].as_str() == Some(binding.map_or(name, |b| b.name.as_str())))
}
pub fn plan(project: &Project, actual: &Snapshot, state: &Bindings, adopt: bool) -> Plan {
    let mut plan = Plan {
        project: project.manifest.project_id.clone(),
        collection: project.manifest.collection.clone(),
        operations: vec![],
        conflicts: vec![],
        warnings: vec![],
    };
    let mut push =
        |kind: &str, resource: String, request: &str, data: Value, before: Value, after: Value| {
            let id = format!("op-{:04}", plan.operations.len() + 1);
            plan.operations.push(Operation {
                id,
                kind: kind.into(),
                resource,
                request: request.into(),
                data,
                before,
                after,
            });
        };
    let collection = &project.manifest.collection;
    let current = actual.collections["currentSceneCollectionName"]
        .as_str()
        .unwrap_or("");
    if current != collection {
        let exists = actual.collections["sceneCollections"]
            .as_array()
            .is_some_and(|v| v.iter().any(|v| v.as_str() == Some(collection)));
        if exists {
            plan.conflicts.push(json!({"code":"collection_not_selected","collection":collection,"message":"Select this collection explicitly before diff/apply; discovery never switches collections"}));
            return plan;
        }
        push(
            "create",
            "collection".into(),
            "CreateSceneCollection",
            json!({"sceneCollectionName":collection}),
            Value::Null,
            json!(collection),
        );
    }
    if project
        .manifest
        .sync
        .as_ref()
        .is_some_and(|s| s.mode == "replace" || s.delete_unmanaged)
    {
        plan.conflicts.push(json!({"code":"destructive_unsupported","message":"Collection replacement and unmanaged deletion are not supported"}));
    }
    let mut observed_names = HashSet::new();
    if actual
        .scenes
        .iter()
        .any(|scene| !observed_names.insert(scene["sceneName"].as_str().unwrap_or("")))
    {
        plan.conflicts.push(json!({"code":"ambiguous_scene_names","message":"OBS returned repeated scene names; select an unambiguous collection/canvas"}));
    }
    // Create/rename every scene before attaching references.
    let mut scene_names = BTreeMap::new();
    for scene in &project.scenes {
        scene_names.insert(scene.id.clone(), scene.name.clone());
        let binding = state.scenes.get(&scene.id);
        let found = if current == collection {
            resolve(
                &actual.scenes,
                binding,
                &scene.name,
                "sceneName",
                "sceneUuid",
            )
            .or_else(|| {
                if adopt {
                    actual
                        .scenes
                        .iter()
                        .find(|value| value["sceneName"] == scene.name)
                } else {
                    None
                }
            })
        } else {
            None
        };
        if let Some(found) = found {
            if binding.is_none() && !adopt {
                plan.conflicts.push(json!({"code":"unmanaged_scene","resource":scene.id,"message":"Name exists without ownership; inspect and use --adopt-existing to manage it"}));
                continue;
            }
            let old = found["sceneName"].as_str().unwrap_or("");
            if old != scene.name && actual.scenes.iter().any(|s| s["sceneName"] == scene.name) {
                plan.conflicts
                    .push(json!({"code":"scene_name_collision","resource":scene.id}));
                continue;
            }
            push(
                if old == scene.name { "noop" } else { "update" },
                format!("scene/{}", scene.id),
                "SetSceneName",
                json!({"sceneName":old,"newSceneName":scene.name}),
                json!(old),
                json!(scene.name),
            );
        } else {
            if actual
                .scenes
                .iter()
                .any(|value| value["sceneName"] == scene.name)
            {
                plan.conflicts.push(json!({"code":"stale_scene_binding","resource":scene.id,"message":"Bound UUID disappeared but the desired name exists. Resolve state ownership explicitly."}));
                continue;
            }
            push(
                "create",
                format!("scene/{}", scene.id),
                "CreateScene",
                json!({"sceneName":scene.name}),
                Value::Null,
                json!(scene.name),
            );
        }
    }
    let mut input_declarations: BTreeMap<String, (String, Option<Value>)> = BTreeMap::new();
    for scene in &project.scenes {
        for item in &scene.items {
            if item.kind == "input" {
                let name = item.input_name.as_ref().expect("validated");
                let kind = item.input_kind.as_ref().expect("validated");
                if let Some(previous) = input_declarations.insert(
                    item.input_id.clone().unwrap_or_else(|| name.clone()),
                    (format!("{name}/{kind}"), item.settings.clone()),
                ) {
                    if previous != (format!("{name}/{kind}"), item.settings.clone()) {
                        plan.conflicts
                            .push(json!({"code":"input_definition_collision","resource":name}));
                    }
                }
            }
        }
    }
    let mut planned_inputs = HashSet::new();
    for scene in &project.scenes {
        let old_scene = resolve(
            &actual.scenes,
            state.scenes.get(&scene.id),
            &scene.name,
            "sceneName",
            "sceneUuid",
        )
        .and_then(|s| s["sceneName"].as_str())
        .unwrap_or(&scene.name);
        let items = actual
            .items
            .get(old_scene)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        for item in &scene.items {
            let source = if let Some(id) = &item.scene_id {
                scene_names.get(id).expect("validated")
            } else {
                item.input_name.as_ref().expect("validated")
            };
            let input_id = item.input_id.as_deref().unwrap_or(source);
            if item.kind != "scene_reference" && planned_inputs.insert(input_id.to_string()) {
                let found = resolve(
                    &actual.inputs,
                    state.inputs.get(input_id),
                    source,
                    "inputName",
                    "inputUuid",
                )
                .or_else(|| {
                    if adopt {
                        actual
                            .inputs
                            .iter()
                            .find(|value| value["inputName"] == *source)
                    } else {
                        None
                    }
                });
                if item.kind == "existing_input" {
                    if found.is_none() {
                        plan.conflicts
                            .push(json!({"code":"missing_external_input","resource":source}));
                    }
                } else if let Some(found) = found {
                    let old_name = found["inputName"].as_str().unwrap_or(source);
                    if old_name != source
                        && actual
                            .inputs
                            .iter()
                            .any(|input| input["inputName"] == *source)
                    {
                        plan.conflicts
                            .push(json!({"code":"input_name_collision","resource":input_id}));
                        continue;
                    }
                    if old_name != source {
                        push(
                            "update",
                            format!("input/{input_id}"),
                            "SetInputName",
                            json!({"inputName":old_name,"newInputName":source}),
                            json!(old_name),
                            json!(source),
                        );
                    }
                    if found["inputKind"].as_str() != item.input_kind.as_deref() {
                        plan.conflicts.push(json!({"code":"input_kind_conflict","resource":source,"message":"Input kind cannot be changed safely; choose a new input name"}));
                    } else if !state.inputs.contains_key(input_id) && !adopt {
                        plan.conflicts
                            .push(json!({"code":"unmanaged_input","resource":source}));
                    } else if let Some(settings) = &item.settings {
                        let before = actual
                            .settings
                            .get(old_name)
                            .cloned()
                            .unwrap_or(Value::Null);
                        push(
                            if patch_diff(settings, &before) {
                                "update"
                            } else {
                                "noop"
                            },
                            format!("input/{input_id}"),
                            "SetInputSettings",
                            json!({"inputName":source,"inputSettings":settings,"overlay":true}),
                            json!({"redacted":true}),
                            settings.clone(),
                        );
                    }
                } else {
                    if !actual
                        .kinds
                        .iter()
                        .any(|kind| Some(kind) == item.input_kind.as_ref())
                    {
                        plan.conflicts.push(json!({"code":"unsupported_input_kind","resource":source,"inputKind":item.input_kind}));
                    }
                    push(
                        "create",
                        format!("input/{input_id}"),
                        "CreateInput",
                        json!({"sceneName":scene.name,"inputName":source,"inputKind":item.input_kind,"inputSettings":item.settings.clone().unwrap_or(json!({})),"sceneItemEnabled":item.enabled.unwrap_or(true)}),
                        Value::Null,
                        json!(source),
                    );
                }
            }
            let resource = format!("item/{}/{}", scene.id, item.id);
            let binding = state.items.get(&resource);
            let matches = items
                .iter()
                .filter(|i| i["sourceName"].as_str() == Some(source))
                .collect::<Vec<_>>();
            let found = binding
                .and_then(|id| {
                    items
                        .iter()
                        .find(|i| i["sceneItemId"].as_i64() == Some(*id))
                })
                .or_else(|| {
                    if adopt && matches.len() == 1 {
                        Some(matches[0])
                    } else {
                        None
                    }
                });
            if let Some(existing) = found {
                let expected_previous = if let Some(child) = item.scene_id.as_deref() {
                    state
                        .scenes
                        .get(child)
                        .and_then(|binding| {
                            resolve(
                                &actual.scenes,
                                Some(binding),
                                source,
                                "sceneName",
                                "sceneUuid",
                            )
                        })
                        .and_then(|value| value["sceneName"].as_str())
                } else {
                    state
                        .inputs
                        .get(input_id)
                        .and_then(|binding| {
                            resolve(
                                &actual.inputs,
                                Some(binding),
                                source,
                                "inputName",
                                "inputUuid",
                            )
                        })
                        .and_then(|value| value["inputName"].as_str())
                };
                if existing["sourceName"] != *source
                    && !expected_previous.is_some_and(|previous| existing["sourceName"] == previous)
                {
                    plan.conflicts.push(json!({"code":"scene_item_source_changed","resource":resource,"message":"Managed item points to a different source. Use a new item ID or explicitly remove the old managed instance."}));
                    continue;
                }
            }
            if found.is_none() && !matches.is_empty() {
                plan.conflicts
                    .push(json!({"code":"unmanaged_or_ambiguous_item","resource":resource}));
                continue;
            }
            let predicted_count = items.len()
                + scene
                    .items
                    .iter()
                    .filter(|candidate| {
                        let key = format!("item/{}/{}", scene.id, candidate.id);
                        !state.items.get(&key).is_some_and(|id| {
                            items
                                .iter()
                                .any(|actual| actual["sceneItemId"].as_i64() == Some(*id))
                        })
                    })
                    .count();
            if item
                .index
                .is_some_and(|index| index as usize >= predicted_count)
            {
                plan.conflicts
                    .push(json!({"code":"scene_item_index_out_of_range","resource":resource}));
            }
            let scene_item_id = found.and_then(|i| i["sceneItemId"].as_i64());
            push(
                if found.is_some() { "noop" } else { "create" },
                resource.clone(),
                "CreateSceneItem",
                json!({"sceneName":scene.name,"sourceName":source,"sceneItemEnabled":item.enabled.unwrap_or(true)}),
                found.cloned().unwrap_or(Value::Null),
                json!(source),
            );
            for (field, request, data_field) in [
                ("enabled", "SetSceneItemEnabled", "sceneItemEnabled"),
                ("transform", "SetSceneItemTransform", "sceneItemTransform"),
                ("index", "SetSceneItemIndex", "sceneItemIndex"),
            ] {
                let desired = match field {
                    "enabled" => item.enabled.map(Value::Bool),
                    "transform" => item.transform.clone(),
                    _ => item.index.map(|v| json!(v)),
                };
                if let Some(desired) = desired {
                    let before = found.map(|v| v[data_field].clone()).unwrap_or(Value::Null);
                    let mut data = json!({"sceneName":scene.name,"sceneItemId":scene_item_id,"sourceName":source});
                    data[data_field] = desired.clone();
                    push(
                        if patch_diff(&desired, &before) {
                            "update"
                        } else {
                            "noop"
                        },
                        resource.clone(),
                        request,
                        data,
                        before,
                        desired,
                    );
                }
            }
        }
    }
    let supported = actual.version["availableRequests"].as_array();
    for op in &plan.operations {
        if op.kind != "noop"
            && !supported.is_some_and(|list| list.iter().any(|v| v.as_str() == Some(&op.request)))
        {
            plan.conflicts
                .push(json!({"code":"unsupported_request","operation":op.id,"request":op.request}));
        }
    }
    if project
        .manifest
        .sync
        .as_ref()
        .is_some_and(|s| s.mode == "managed")
    {
        plan.warnings.push(
            "Managed mode preserves stale resources; use --prune-managed --allow-destructive to preview/delete stale UUID-bound resources."
                .into(),
        );
    }
    plan
}
