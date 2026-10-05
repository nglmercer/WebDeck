use super::*;

pub(super) fn resolve_request(a: &App, mut r: CommandRequest) -> Result<CommandRequest> {
    for _ in 0..=8 {
        let Command::Button { button_id } = &r.command else {
            return Ok(r);
        };
        let c = a.config.snapshot()?.config;
        r.command = c
            .layout
            .folders
            .iter()
            .flat_map(|f| &f.buttons)
            .find(|b| &b.id == button_id)
            .and_then(|b| domain::action_command(&b.action))
            .ok_or_else(Error::invalid)?;
    }
    Err(Error::new(
        ErrorCode::InvalidInput,
        "Button references exceed nesting limit",
    ))
}

pub(super) async fn resolve_async(a: &App, request: CommandRequest) -> Result<CommandRequest> {
    if !matches!(request.command, Command::Button { .. }) {
        return Ok(request);
    }
    let owner = a.clone();
    blocking(a.io.clone(), move || resolve_request(&owner, request)).await
}

pub(super) async fn catalog(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<Value>> {
    i.require(Capability::Read)?;
    let owner = a.clone();
    let snapshot = blocking(a.queries.clone(), move || owner.executor.management(None))
        .await
        .ok();
    let mut catalog: Vec<Value> =
        serde_json::from_str(include_str!("../../contracts/catalog.json"))
            .expect("generated catalog");
    if let Some(snapshot) = &snapshot {
        catalog.retain(|c| {
            snapshot["commands"]
                .as_array()
                .is_some_and(|items| items.iter().any(|entry| entry["id"] == c["id"]))
        });
    }
    let disabled = snapshot
        .as_ref()
        .and_then(|v| v["disabled_plugins"].as_array())
        .cloned()
        .unwrap_or_default();
    let plugins = snapshot
        .and_then(|v| serde_json::from_value::<Vec<PluginManifest>>(v["plugins"].clone()).ok())
        .unwrap_or_else(|| a.plugins.as_ref().clone());
    let disabled_ids = disabled
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let mut automation = crate::automation::collect(&plugins, &disabled_ids)?;
    automation.integrations.retain(|definition| {
        i.capabilities.contains(&Capability::Settings)
            && definition
                .probe
                .as_ref()
                .is_none_or(|c| crate::automation::command_allowed(c, &i.capabilities, &plugins))
    });
    automation.button_recipes.retain(|recipe| {
        crate::automation::command_allowed(&recipe.discovery, &i.capabilities, &plugins)
            && crate::automation::command_allowed(&recipe.command, &i.capabilities, &plugins)
    });
    let plugins = plugins
        .into_iter()
        .filter_map(|mut p| {
            if disabled.contains(&json!(p.id)) || !i.capabilities.contains(&Capability::Plugin) {
                return None;
            }
            p.actions.retain(|action| {
                action
                    .capabilities
                    .iter()
                    .all(|c| i.capabilities.contains(c))
            });
            (!p.actions.is_empty()).then_some(p)
        })
        .collect::<Vec<_>>();
    Ok(Json(
        json!({"api_version":2,"automation":automation,"plugins":plugins,"commands":catalog.into_iter().filter(|c|serde_json::from_value::<Capability>(c["capability"].clone()).is_ok_and(|c|i.capabilities.contains(&c))).collect::<Vec<_>>()}),
    ))
}
pub(super) async fn runtime_status(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<Value>> {
    i.local()?;
    i.require(Capability::Settings)?;
    Ok(Json(
        blocking(a.queries.clone(), move || a.executor.management(None)).await?,
    ))
}
pub(super) async fn runtime_reload(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<Value>> {
    i.local()?;
    i.require(Capability::Settings)?;
    Ok(Json(
        blocking(a.io.clone(), move || {
            let plugins = crate::runtime::plugins::load_plugins(&a.assets)?;
            let response = a.executor.management(Some(plugins))?;
            a.integration_health
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .clear();
            Ok(response)
        })
        .await?,
    ))
}

pub(super) async fn plugin_enabled(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(state): Json<Value>,
) -> Result<Json<Value>> {
    i.local()?;
    i.require(Capability::Settings)?;
    domain::validate("PluginState", &state)?;
    let enabled = state["enabled"].as_bool().ok_or_else(Error::invalid)?;
    Ok(Json(
        blocking(a.io.clone(), move || {
            let response = a.executor.plugin_enabled(&id, enabled)?;
            a.integration_health
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .clear();
            Ok(response)
        })
        .await?,
    ))
}
pub(super) fn command_event(id: String, result: Result<Value>) -> Value {
    match result {
        Ok(result) => json!({"api_version":2,"request_id":id,"state":"completed","result":result}),
        Err(e) => {
            json!({"api_version":2,"request_id":id,"state":"failed","code":e.code,"message":e.message})
        }
    }
}
pub(super) async fn command(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
    Json(r): Json<CommandRequest>,
) -> Response {
    let id = r.request_id.clone();
    let result = match resolve_async(&a, r).await {
        Ok(r) => a.executor.execute(r, i.capabilities, || {}).await,
        Err(e) => Err(e),
    };
    let status = match &result {
        Ok(_) => StatusCode::OK,
        Err(e) => match e.code {
            ErrorCode::Forbidden => StatusCode::FORBIDDEN,
            ErrorCode::CapacityExhausted => StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::ShuttingDown => StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::InvalidInput => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        },
    };
    (status, Json(command_event(id, result))).into_response()
}

pub(super) async fn usage(
    State(a): State<App>,
    axum::Extension(i): axum::Extension<Identity>,
) -> Result<Json<Value>> {
    i.require(Capability::Read)?;
    let v = blocking(a.queries.clone(), || Ok(crate::capabilities::usage())).await?;
    Ok(Json(json!({"api_version":2,"usage":v})))
}
