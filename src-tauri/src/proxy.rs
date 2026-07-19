use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect},
    routing::get,
    Router,
};
use serde::Serialize;
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;

#[derive(Clone)]
struct ProxyContext {
    console_origin: String,
    release_version: String,
    client: reqwest::Client,
    access_token: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResolveRequest {
    release_version: String,
    relative_path: String,
    category_id: Option<String>,
}

pub async fn start(
    console_origin: String,
    release_version: String,
    access_token: Option<String>,
) -> Result<String, String> {
    let context = Arc::new(ProxyContext {
        console_origin,
        release_version,
        client: reqwest::Client::new(),
        access_token,
    });
    let app = Router::new()
        .route("/client/{*path}", get(resolve_client))
        .route("/addons/{category_id}/{*path}", get(resolve_addon))
        .with_state(context);
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|error| error.to_string())?;
    let address: SocketAddr = listener.local_addr().map_err(|error| error.to_string())?;
    tauri::async_runtime::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Ok(format!("http://{address}/client/"))
}

async fn resolve_client(
    State(context): State<Arc<ProxyContext>>,
    Path(path): Path<String>,
) -> Result<impl IntoResponse, (axum::http::StatusCode, String)> {
    resolve_file(context, path, None).await
}

async fn resolve_addon(
    State(context): State<Arc<ProxyContext>>,
    Path((category_id, path)): Path<(String, String)>,
) -> Result<impl IntoResponse, (axum::http::StatusCode, String)> {
    resolve_file(context, path, Some(category_id)).await
}

async fn resolve_file(
    context: Arc<ProxyContext>,
    path: String,
    category_id: Option<String>,
) -> Result<impl IntoResponse, (axum::http::StatusCode, String)> {
    let endpoint = format!(
        "{}/api/download-resolutions",
        context.console_origin.trim_end_matches('/')
    );
    let request = context.client.post(endpoint).json(&ResolveRequest {
        release_version: context.release_version.clone(),
        relative_path: path,
        category_id,
    });
    let response = if let Some(access_token) = &context.access_token {
        request.bearer_auth(access_token)
    } else {
        request
    }
    .send()
    .await
    .map_err(internal)?;
    if !response.status().is_success() {
        return Err((
            axum::http::StatusCode::BAD_GATEWAY,
            "Console refused protected file resolution".into(),
        ));
    }
    let resolved = response
        .json::<serde_json::Value>()
        .await
        .map_err(internal)?;
    let url = resolved
        .get("url")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            (
                axum::http::StatusCode::BAD_GATEWAY,
                "Console returned invalid resolution".into(),
            )
        })?;
    Ok(Redirect::temporary(url))
}

fn internal(error: reqwest::Error) -> (axum::http::StatusCode, String) {
    (axum::http::StatusCode::BAD_GATEWAY, error.to_string())
}
