use super::*;
use socketioxide::{
    extract::{Data, SocketRef},
    layer::SocketIoLayer,
    SocketIo,
};
pub(super) fn layer(app: App) -> SocketIoLayer {
    let (layer, io) = SocketIo::new_layer();
    io.ns("/v2", move |socket: SocketRef, Data(auth): Data<Value>| {
        on_connect(app.clone(), socket, auth)
    });
    layer
}
async fn on_connect(app: App, socket: SocketRef, auth: Value) {
    let peer = socket
        .req_parts()
        .extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map(|p| p.0.ip());
    let Some(peer) = peer else {
        let _ = socket.disconnect();
        return;
    };
    if !auth.is_object() {
        let _ = socket.disconnect();
        return;
    }
    let token = match auth.get("token") {
        None => super::token(&socket.req_parts().headers).unwrap_or_else(|_| Some(String::new())),
        Some(Value::String(t)) => Some(t.clone()),
        Some(_) => Some(String::new()),
    };
    if authorize(&app, peer, token.clone()).await.is_err() {
        let _ = socket.disconnect();
        return;
    }
    let commands = app.clone();
    let credential = token.clone();
    socket.on("command", move |s: SocketRef, Data(v): Data<Value>| {
        invoke(commands.clone(), s, credential.clone(), peer, v)
    });
    socket.on("usage", move |s: SocketRef, Data(v): Data<Value>| {
        usage(app.clone(), s, token.clone(), peer, v)
    });
}
async fn invoke(app: App, socket: SocketRef, token: Option<String>, peer: IpAddr, v: Value) {
    let id = v["request_id"].as_str().unwrap_or("").to_string();
    let result = async {
        domain::validate("CommandRequest", &v)?;
        let r: CommandRequest = serde_json::from_value(v).map_err(|_| Error::invalid())?;
        let caps = authorize(&app, peer, token).await?.capabilities;
        let r = resolve_async(&app, r).await?;
        let observer = socket.clone();
        let correlation = id.clone();
        app.executor
            .execute(r, caps, move || {
                let _ = observer.emit(
                    "command_result",
                    &json!({"api_version":2,"request_id":correlation,"state":"accepted"}),
                );
            })
            .await
    }
    .await;
    let _ = socket.emit("command_result", &command_event(id, result));
}
async fn usage(app: App, socket: SocketRef, token: Option<String>, peer: IpAddr, v: Value) {
    let id = v["request_id"].as_str().unwrap_or("");
    if id.is_empty() || id.len() > 128 {
        return;
    }
    if authorize(&app, peer, token)
        .await
        .is_ok_and(|c| c.capabilities.contains(&Capability::Read))
    {
        if let Ok(value) = blocking(app.queries.clone(), || Ok(crate::native::usage())).await {
            let _ = socket.emit(
                "usage_result",
                &json!({"api_version":2,"request_id":id,"usage":value}),
            );
        }
    } else {
        let _ = socket.disconnect();
    }
}
