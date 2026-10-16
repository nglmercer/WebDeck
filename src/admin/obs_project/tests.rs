use super::*;
fn example() -> Project {
    load(Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/obs-project"
    )))
    .unwrap()
}
fn snapshot(project: &Project) -> Snapshot {
    Snapshot {
        version: json!({"availableRequests":["CreateSceneCollection","CreateScene","CreateInput","CreateSceneItem","SetSceneItemEnabled","SetSceneItemTransform","SetSceneName","SetInputSettings","SetSceneItemIndex"]}),
        collections: json!({"currentSceneCollectionName":project.manifest.collection,"sceneCollections":[project.manifest.collection]}),
        ..Default::default()
    }
}
#[test]
fn example_and_schema_are_valid() {
    let project = example();
    assert_eq!(project.scenes.len(), 5);
    assert!(schema()["$defs"]["Scene"].is_object());
}
#[test]
fn planner_orders_scenes_before_items_and_reuses_overlay() {
    let project = example();
    let actual = snapshot(&project);
    let plan = plan(&project, &actual, &Bindings::default(), false);
    assert!(plan.conflicts.is_empty(), "{:?}", plan.conflicts);
    assert_eq!(
        plan.operations
            .iter()
            .filter(|o| o.request == "CreateScene")
            .count(),
        5
    );
    assert_eq!(
        plan.operations
            .iter()
            .filter(|o| o.request == "CreateSceneItem")
            .count(),
        4
    );
    assert!(plan.operations[..5]
        .iter()
        .all(|o| o.request == "CreateScene"));
    assert!(plan
        .operations
        .iter()
        .filter(|o| o.request == "CreateSceneItem")
        .all(|o| o.data["sourceName"] == "CommonOverlay"));
}
#[test]
fn second_plan_is_noop_and_uuid_rename_preserves_identity() {
    let project = example();
    let mut actual = snapshot(&project);
    let mut state = Bindings::default();
    for (index, scene) in project.scenes.iter().enumerate() {
        let uuid = format!("uuid-{index}");
        actual
            .scenes
            .push(json!({"sceneName":scene.name,"sceneUuid":uuid}));
        state.scenes.insert(
            scene.id.clone(),
            Binding {
                name: scene.name.clone(),
                uuid: Some(uuid),
            },
        );
        if !scene.items.is_empty() {
            actual.items.insert(
                scene.name.clone(),
                vec![json!({"sourceName":"CommonOverlay","sceneItemId":1,"sceneItemEnabled":true})],
            );
            state.items.insert(format!("item/{}/overlay", scene.id), 1);
        }
    }
    let noop = plan(&project, &actual, &state, false);
    assert!(noop.conflicts.is_empty());
    assert!(noop.operations.iter().all(|o| o.kind == "noop"));
    actual.scenes[1]["sceneName"] = json!("Old Name");
    let renamed = plan(&project, &actual, &state, false);
    assert!(renamed.conflicts.is_empty());
    assert!(renamed
        .operations
        .iter()
        .any(|o| o.request == "SetSceneName"
            && o.kind == "update"
            && o.data["sceneName"] == "Old Name"));
}
#[test]
fn bound_item_source_change_is_a_conflict_instead_of_a_silent_retarget() {
    let project = example();
    let mut actual = snapshot(&project);
    let mut state = Bindings::default();
    actual
        .scenes
        .push(json!({"sceneName":"Starting","sceneUuid":"starting-uuid"}));
    actual.items.insert(
        "Starting".into(),
        vec![json!({"sceneItemId":7,"sourceName":"Unrelated Camera","sceneItemEnabled":true})],
    );
    state.scenes.insert(
        "starting".into(),
        Binding {
            name: "Starting".into(),
            uuid: Some("starting-uuid".into()),
        },
    );
    state.items.insert("item/starting/overlay".into(), 7);
    let planned = plan(&project, &actual, &state, false);
    assert!(planned
        .conflicts
        .iter()
        .any(|conflict| conflict["code"] == "scene_item_source_changed"));
}
#[test]
fn unmanaged_scenes_require_adoption_and_are_never_deleted() {
    let project = example();
    let mut actual = snapshot(&project);
    actual
        .scenes
        .push(json!({"sceneName":"Starting","sceneUuid":"external"}));
    let planned = plan(&project, &actual, &Bindings::default(), false);
    assert!(!planned.conflicts.is_empty());
    assert!(!planned.operations.iter().any(|o| o.kind == "delete"));
    assert!(plan(&project, &actual, &Bindings::default(), true)
        .conflicts
        .is_empty());
}
#[test]
fn unsafe_paths_cycles_unknown_fields_and_duplicates_fail_offline() {
    let root = std::env::temp_dir().join(format!(
        "webdeck-project-test-{}",
        crate::domain::id().unwrap()
    ));
    init(&root).unwrap();
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(root.join("project.json")).unwrap()).unwrap();
    for malformed in [
        json!({"id":"starting","name":"Starting","items":[{"id":"cycle","kind":"scene_reference","sceneId":"starting"}]}),
        json!({"id":"starting","name":"Starting","items":[],"typo":true}),
        json!({"id":"starting","name":"Starting","items":[{"id":"dup","kind":"scene_reference","sceneId":"common-overlay"},{"id":"dup","kind":"scene_reference","sceneId":"common-overlay"}]}),
    ] {
        std::fs::write(root.join("scenes/starting.json"), malformed.to_string()).unwrap();
        assert!(load(&root).is_err());
    }
    let mut escape = manifest;
    escape["scenes"] = json!(["../escape.json"]);
    std::fs::write(root.join("project.json"), escape.to_string()).unwrap();
    assert!(load(&root).is_err());
    assert!(init(&root).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn prune_requires_mode_ownership_and_preserves_inbound_unmanaged_references() {
    let mut project = example();
    project.manifest.sync = Some(Sync {
        mode: "managed".into(),
        delete_unmanaged: false,
    });
    let mut actual = snapshot(&project);
    actual.version["availableRequests"]
        .as_array_mut()
        .unwrap()
        .extend([
            json!("RemoveScene"),
            json!("RemoveSceneItem"),
            json!("RemoveInput"),
        ]);
    actual
        .scenes
        .push(json!({"sceneName":"Old Managed","sceneUuid":"old-uuid"}));
    let mut state = Bindings::default();
    state.scenes.insert(
        "old".into(),
        Binding {
            name: "Old Managed".into(),
            uuid: Some("old-uuid".into()),
        },
    );
    let mut prune = plan(&project, &actual, &state, false);
    super::prune::plan_prune(&project, &actual, &state, &mut prune);
    assert!(prune
        .operations
        .iter()
        .any(|op| op.request == "RemoveScene"));
    actual.items.insert(
        "Unmanaged Parent".into(),
        vec![json!({"sourceName":"Old Managed","sceneItemId":42})],
    );
    let mut blocked = plan(&project, &actual, &state, false);
    super::prune::plan_prune(&project, &actual, &state, &mut blocked);
    assert!(blocked
        .conflicts
        .iter()
        .any(|c| c["code"] == "managed_scene_still_referenced"));
    assert!(!blocked.operations.iter().any(|op| op.kind == "delete"));
    state.scenes.get_mut("old").unwrap().uuid = None;
    let mut unverified = plan(&project, &actual, &state, false);
    super::prune::plan_prune(&project, &actual, &state, &mut unverified);
    assert!(unverified
        .conflicts
        .iter()
        .any(|c| c["code"] == "prune_identity_unverified"));
}
#[test]
fn shared_instance_template_only_changes_managed_instances_and_honors_overrides() {
    let mut project = example();
    let mut actual = snapshot(&project);
    let mut state = Bindings::default();
    for scene in &mut project.scenes {
        for item in &mut scene.items {
            state
                .items
                .insert(format!("item/{}/{}", scene.id, item.id), 1);
            state.item_sources.insert(
                format!("item/{}/{}", scene.id, item.id),
                "CommonOverlay".into(),
            );
            actual.items.insert(scene.name.clone(),vec![json!({"sceneItemId":1,"sourceName":"CommonOverlay","sceneItemEnabled":true,"sceneItemTransform":{"positionX":5,"positionY":2}})]);
            if scene.id == "gameplay" {
                item.transform = Some(json!({"positionX":99}));
            }
        }
    }
    let properties = InstanceProperties {
        enabled: Some(false),
        transform: Some(json!({"positionX":10})),
        index: None,
    };
    let planned = plan_instances(&project, &actual, &state, "common-overlay", &properties);
    assert!(planned.conflicts.is_empty());
    assert_eq!(planned.operations.len(), 8);
    let gameplay = planned
        .operations
        .iter()
        .find(|op| op.resource == "item/gameplay/overlay" && op.request == "SetSceneItemTransform")
        .unwrap();
    assert_eq!(gameplay.after["positionX"], 99);
    // Each parent explicitly declares enabled=true, so the template preserves it.
    assert!(planned
        .operations
        .iter()
        .filter(|op| op.request == "SetSceneItemEnabled")
        .all(|op| op.after == true && op.kind == "noop"));
    state.items.clear();
    assert!(
        !plan_instances(&project, &actual, &state, "common-overlay", &properties)
            .conflicts
            .is_empty()
    );
}
