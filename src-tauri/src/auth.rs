use crate::{
    contracts::UpdaterStatus,
    state::{DesktopIdentity, UpdaterState},
    windows,
};
use keyring::Entry;
use serde::Deserialize;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, State};
use url::Url;

const DESKTOP_REFRESH_CREDENTIAL_SERVICE: &str = "top.aurlemon.hydcraft.updater";
const DESKTOP_REFRESH_CREDENTIAL_ACCOUNT: &str = "desktop-refresh-token";

#[tauri::command]
pub fn desktop_login_url(state: State<'_, UpdaterState>) -> String {
    desktop_login_url_value(&state.console_origin)
}

pub(crate) fn desktop_login_url_value(console_origin: &str) -> String {
    format!(
        "{}/api/oidc/hydcraft/login?desktop_redirect_uri=hydcraft-updater%3A%2F%2Fauth%2Fcallback",
        console_origin.trim_end_matches('/')
    )
}

#[tauri::command]
pub async fn exchange_desktop_code(
    code: String,
    state: State<'_, UpdaterState>,
) -> Result<UpdaterStatus, String> {
    exchange_desktop_code_value(code, state.inner().clone()).await
}

pub(crate) async fn exchange_desktop_code_value(
    code: String,
    state: UpdaterState,
) -> Result<UpdaterStatus, String> {
    if code.trim().is_empty() {
        return Err("Desktop authorization code is empty".into());
    }
    let bundle = request_desktop_token_bundle(
        format!(
            "{}/api/desktop-auth/exchange",
            state.console_origin.trim_end_matches('/')
        ),
        serde_json::json!({ "code": code }),
    )
    .await?;
    if let Err(error) = save_desktop_refresh_token(&bundle.refresh_token) {
        let _ = revoke_desktop_session(&state.console_origin, &bundle.refresh_token).await;
        return Err(error);
    }
    set_access_token(&state, &bundle).await;
    load_desktop_identity(&state).await?;
    state
        .set_status("authenticated", "HydCraft 账户已登录", None)
        .await;
    Ok(state.status.read().await.clone())
}

#[derive(Deserialize)]
struct DesktopProfileResponse {
    identity: Option<DesktopIdentity>,
}

pub(crate) async fn load_desktop_identity(state: &UpdaterState) -> Result<(), String> {
    let token = state.access_token.read().await.clone();
    let Some(token) = token else { return Ok(()) };
    let response = reqwest::Client::new()
        .get(format!(
            "{}/api/auth/me",
            state.console_origin.trim_end_matches('/')
        ))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "Unable to load HydCraft profile: {}",
            response.status()
        ));
    }
    *state.identity.write().await = response
        .json::<DesktopProfileResponse>()
        .await
        .map_err(|error| error.to_string())?
        .identity;
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopTokenBundle {
    access_token: String,
    refresh_token: String,
    #[serde(default)]
    expires_in: Option<u64>,
}

async fn set_access_token(state: &UpdaterState, bundle: &DesktopTokenBundle) {
    *state.access_token.write().await = Some(bundle.access_token.clone());
    *state.access_token_expires_at.write().await = bundle
        .expires_in
        .map(|seconds| Instant::now() + Duration::from_secs(seconds));
}

pub(crate) async fn ensure_desktop_session(state: &UpdaterState) -> Result<(), String> {
    let _refresh_guard = state.auth_refresh_lock.lock().await;
    let should_refresh = {
        let access_token = state.access_token.read().await;
        let expires_at = state.access_token_expires_at.read().await;
        access_token.is_some()
            && expires_at
                .map(|value| value <= Instant::now() + Duration::from_secs(30))
                .unwrap_or(true)
    };
    if !should_refresh {
        return Ok(());
    }

    if restore_desktop_session(state.clone()).await? {
        return Ok(());
    }

    *state.access_token.write().await = None;
    *state.access_token_expires_at.write().await = None;
    *state.identity.write().await = None;
    Ok(())
}

async fn request_desktop_token_bundle(
    endpoint: String,
    payload: serde_json::Value,
) -> Result<DesktopTokenBundle, String> {
    let response = reqwest::Client::new()
        .post(endpoint)
        .json(&payload)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "Desktop authorization failed: {}",
            response.status()
        ));
    }
    response.json().await.map_err(|error| error.to_string())
}

fn desktop_refresh_entry() -> Result<Entry, String> {
    Entry::new(
        DESKTOP_REFRESH_CREDENTIAL_SERVICE,
        DESKTOP_REFRESH_CREDENTIAL_ACCOUNT,
    )
    .map_err(|error| error.to_string())
}

fn load_desktop_refresh_token() -> Option<String> {
    desktop_refresh_entry().ok()?.get_password().ok()
}

fn save_desktop_refresh_token(refresh_token: &str) -> Result<(), String> {
    desktop_refresh_entry()?
        .set_password(refresh_token)
        .map_err(|error| error.to_string())
}

fn clear_desktop_refresh_token() {
    if let Ok(entry) = desktop_refresh_entry() {
        let _ = entry.delete_credential();
    }
}

async fn revoke_desktop_session(console_origin: &str, refresh_token: &str) -> Result<(), String> {
    reqwest::Client::new()
        .post(format!(
            "{}/api/desktop-auth/logout",
            console_origin.trim_end_matches('/')
        ))
        .json(&serde_json::json!({ "refreshToken": refresh_token }))
        .send()
        .await
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub(crate) async fn restore_desktop_session(state: UpdaterState) -> Result<bool, String> {
    let Some(refresh_token) = load_desktop_refresh_token() else {
        return Ok(false);
    };
    let response = reqwest::Client::new()
        .post(format!(
            "{}/api/desktop-auth/refresh",
            state.console_origin.trim_end_matches('/')
        ))
        .json(&serde_json::json!({ "refreshToken": refresh_token }))
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            clear_desktop_refresh_token();
            return Ok(false);
        }
        return Err(format!(
            "Desktop session refresh failed: {}",
            response.status()
        ));
    }
    let bundle = response
        .json::<DesktopTokenBundle>()
        .await
        .map_err(|error| error.to_string())?;
    if let Err(error) = save_desktop_refresh_token(&bundle.refresh_token) {
        let _ = revoke_desktop_session(&state.console_origin, &bundle.refresh_token).await;
        return Err(error);
    }
    set_access_token(&state, &bundle).await;
    load_desktop_identity(&state).await?;
    state
        .set_status("authenticated", "HydCraft 账户已恢复登录", None)
        .await;
    Ok(true)
}

#[tauri::command]
pub async fn logout_desktop(state: State<'_, UpdaterState>) -> Result<(), String> {
    if let Some(refresh_token) = load_desktop_refresh_token() {
        revoke_desktop_session(&state.console_origin, &refresh_token).await?;
    }
    clear_desktop_refresh_token();
    *state.access_token.write().await = None;
    *state.access_token_expires_at.write().await = None;
    *state.identity.write().await = None;
    state
        .set_status("unauthenticated", "HydCraft 账户已退出登录", None)
        .await;
    Ok(())
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopAuthEvent {
    pub(crate) phase: String,
    pub(crate) message: String,
}

fn parse_desktop_callback(url: &Url) -> Result<String, String> {
    if url.scheme() != "hydcraft-updater"
        || url.host_str() != Some("auth")
        || url.path() != "/callback"
    {
        return Err("Unsupported HydCraft desktop callback URL".into());
    }

    let codes = url
        .query_pairs()
        .filter_map(|(key, value)| (key == "code").then_some(value.into_owned()))
        .collect::<Vec<_>>();
    match codes.as_slice() {
        [code] if !code.trim().is_empty() => Ok(code.clone()),
        _ => Err("HydCraft desktop callback must contain exactly one code".into()),
    }
}

pub(crate) async fn process_deep_links(app: AppHandle, state: UpdaterState, urls: Vec<Url>) {
    for url in urls {
        let result = async {
            let code = parse_desktop_callback(&url)?;
            exchange_desktop_code_value(code, state.clone()).await?;
            Ok::<(), String>(())
        }
        .await;

        match result {
            Ok(()) => {
                let _ = windows::show_auth_window(&app);
                let _ = app.emit(
                    "desktop-auth-result",
                    DesktopAuthEvent {
                        phase: "verified".into(),
                        message: "验证成功".into(),
                    },
                );
            }
            Err(message) => {
                state
                    .set_status(
                        "authentication-failed",
                        &format!("HydCraft 账户登录失败：{message}"),
                        None,
                    )
                    .await;
                let _ = windows::show_auth_window(&app);
                let _ = app.emit(
                    "desktop-auth-result",
                    DesktopAuthEvent {
                        phase: "failed".into(),
                        message,
                    },
                );
            }
        }
    }
}

#[tauri::command]
pub fn start_desktop_login(app: AppHandle, state: State<'_, UpdaterState>) -> Result<(), String> {
    windows::show_auth_window(&app)?;
    let url = desktop_login_url_value(&state.console_origin);
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(url, None::<String>)
        .map_err(|error| error.to_string())?;
    app.emit(
        "desktop-auth-result",
        DesktopAuthEvent {
            phase: "browser-opened".into(),
            message: "浏览器已打开，请完成登录与授权".into(),
        },
    )
    .map_err(|error| error.to_string())
}
