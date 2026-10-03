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
            .and_then(|b| {
                if let ButtonAction::Command { command } = &b.action {
                    Some(command.clone())
                } else {
                    None
                }
            })
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
    let catalog: Vec<Value> = serde_json::from_str(include_str!("../../contracts/catalog.json"))
        .expect("generated catalog");
    Ok(Json(
        json!({"api_version":2,"plugins":a.plugins.iter().filter(|p|p.actions.iter().any(|a|a.capabilities.iter().all(|c|i.capabilities.contains(c)))).collect::<Vec<_>>(),"commands":catalog.into_iter().filter(|c|serde_json::from_value::<Capability>(c["capability"].clone()).is_ok_and(|c|i.capabilities.contains(&c))).collect::<Vec<_>>()}),
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
    let v = blocking(a.queries.clone(), || Ok(crate::native::usage())).await?;
    Ok(Json(json!({"api_version":2,"usage":v})))
}
