mod contracts;
mod engine;
mod state;

use contracts::{UpdateConflict, UpdaterStatus};
use engine::EngineError;
use keyring::Entry;
use state::{DesktopIdentity, UpdaterState};
use std::{env, path::PathBuf};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_opener::OpenerExt;
use url::Url;

const DESKTOP_REFRESH_CREDENTIAL_SERVICE: &str = "top.aurlemon.hydcraft.updater";
const DESKTOP_REFRESH_CREDENTIAL_ACCOUNT: &str = "desktop-refresh-token";

#[tauri::command]
async fn updater_status(state: State<'_, UpdaterState>) -> Result<UpdaterStatus, String> {
    Ok(state.status.read().await.clone())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdaterContext {
    mode: String,
    game_dir: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ClientInspection {
    version: Option<String>,
    needs_selection: bool,
}

#[tauri::command]
async fn updater_context(state: State<'_, UpdaterState>) -> Result<UpdaterContext, String> {
    Ok(UpdaterContext {
        mode: state.mode.clone(),
        game_dir: state.game_dir.to_string_lossy().into_owned(),
    })
}

#[tauri::command]
async fn client_version_options(
    state: State<'_, UpdaterState>,
) -> Result<Vec<engine::ClientVersionOption>, String> {
    match engine::list_versions(&state).await {
        Ok(options) if !options.is_empty() => Ok(options),
        _ => Ok(engine::fallback_version_options()),
    }
}

#[tauri::command]
async fn download_sources(
    locale: Option<String>,
    state: State<'_, UpdaterState>,
) -> Result<Vec<engine::DownloadSource>, String> {
    engine::list_sources(&state, locale.as_deref().unwrap_or("zh-CN"))
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn inspect_client(state: State<'_, UpdaterState>) -> Result<ClientInspection, String> {
    match engine::inspect_client_version(&state.game_dir) {
        Ok(version) => Ok(ClientInspection {
            needs_selection: version.is_none(),
            version,
        }),
        Err(error) => Err(error.to_string()),
    }
}

#[tauri::command]
fn open_version_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("version")
        .or_else(|| {
            WebviewWindowBuilder::new(&app, "version", WebviewUrl::App("index.html".into()))
                .title("HydCraft 选择客户端版本")
                .inner_size(420.0, 500.0)
                .center()
                .resizable(false)
                .decorations(false)
                .visible(false)
                .build()
                .ok()
        })
        .ok_or_else(|| "无法创建版本选择窗口".to_string())?;
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
}

#[tauri::command]
fn hide_version_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("version") {
        window.hide().map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn desktop_identity(
    state: State<'_, UpdaterState>,
) -> Result<Option<DesktopIdentity>, String> {
    Ok(state.identity.read().await.clone())
}

#[tauri::command]
async fn pending_conflicts(state: State<'_, UpdaterState>) -> Result<Vec<UpdateConflict>, String> {
    Ok(state.conflicts.read().await.clone())
}

#[tauri::command]
async fn resolve_conflicts(
    resolutions: std::collections::HashMap<String, String>,
    state: State<'_, UpdaterState>,
    app: AppHandle,
) -> Result<UpdaterStatus, String> {
    *state.resolutions.write().await = resolutions;
    state.conflicts.write().await.clear();
    let selected = state.selected_version.read().await.clone();
    let source = state.selected_source.read().await.clone();
    tauri::async_runtime::spawn(execute_update(app, state.inner().clone(), selected, source));
    Ok(state.status.read().await.clone())
}

#[tauri::command]
async fn select_current_version(
    version: String,
    state: State<'_, UpdaterState>,
) -> Result<UpdaterStatus, String> {
    let version = if version == "__no-version__" {
        "1.0.0".to_string()
    } else {
        version
    };
    *state.selected_version.write().await = Some(version.clone());
    state
        .set_status("checking-update", "正在校验所选客户端版本", None)
        .await;
    match engine::check_next(&state, &version).await {
        Ok(check) if check.update_available => {
            state
                .set_status("awaiting-update-decision", "发现客户端更新", None)
                .await;
        }
        Ok(_) => {
            state
                .set_status("up-to-date", "客户端已是最新版本", None)
                .await
        }
        Err(error) => {
            state
                .set_status("failed", &format!("无法检查当前客户端版本：{error}"), None)
                .await
        }
    }
    Ok(state.status.read().await.clone())
}

#[tauri::command]
async fn select_download_source(
    source_key: String,
    state: State<'_, UpdaterState>,
) -> Result<(), String> {
    *state.selected_source.write().await = Some(source_key);
    Ok(())
}

#[tauri::command]
async fn begin_update(app: AppHandle, state: State<'_, UpdaterState>) -> Result<(), String> {
    let selected = state.selected_version.read().await.clone();
    let source = state.selected_source.read().await.clone();
    state.set_status("updating", "正在更新客户端", None).await;
    tauri::async_runtime::spawn(execute_update(app, state.inner().clone(), selected, source));
    Ok(())
}

#[tauri::command]
async fn recheck_update(state: State<'_, UpdaterState>) -> Result<(), String> {
    let Some(version) = state.selected_version.read().await.clone() else {
        return Ok(());
    };
    match engine::check_next(&state, &version).await {
        Ok(check) if check.update_available => {
            state
                .set_status("awaiting-update-decision", "发现客户端更新", None)
                .await
        }
        Ok(_) => {
            state
                .set_status("up-to-date", "客户端已是最新版本", None)
                .await
        }
        Err(error) => {
            state
                .set_status("failed", &format!("更新检查失败：{error}"), None)
                .await
        }
    }
    Ok(())
}

#[tauri::command]
async fn skip_update(app: AppHandle, state: State<'_, UpdaterState>) -> Result<(), String> {
    state
        .set_status("deferred", "已选择继续使用当前客户端", None)
        .await;
    if state.mode == "bootstrap" {
        app.exit(0);
    }
    Ok(())
}

#[tauri::command]
async fn launch_client(app: AppHandle, state: State<'_, UpdaterState>) -> Result<(), String> {
    if state.mode != "bootstrap" {
        return Err("BOOTSTRAP_REQUIRED".into());
    }
    app.exit(0);
    Ok(())
}

#[tauri::command]
fn desktop_login_url(state: State<'_, UpdaterState>) -> String {
    desktop_login_url_value(&state.console_origin)
}

fn desktop_login_url_value(console_origin: &str) -> String {
    format!(
        "{}/api/oidc/hydcraft/login?desktop_redirect_uri=hydcraft-updater%3A%2F%2Fauth%2Fcallback",
        console_origin.trim_end_matches('/')
    )
}

#[tauri::command]
async fn exchange_desktop_code(
    code: String,
    state: State<'_, UpdaterState>,
) -> Result<UpdaterStatus, String> {
    exchange_desktop_code_value(code, state.inner().clone()).await
}

async fn exchange_desktop_code_value(
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
    *state.access_token.write().await = Some(bundle.access_token);
    load_desktop_identity(&state).await?;
    state
        .set_status("authenticated", "HydCraft 账户已登录", None)
        .await;
    Ok(state.status.read().await.clone())
}

#[derive(serde::Deserialize)]
struct DesktopProfileResponse {
    identity: Option<DesktopIdentity>,
}

async fn load_desktop_identity(state: &UpdaterState) -> Result<(), String> {
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

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopTokenBundle {
    access_token: String,
    refresh_token: String,
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

async fn restore_desktop_session(state: UpdaterState) -> Result<bool, String> {
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
    *state.access_token.write().await = Some(bundle.access_token);
    load_desktop_identity(&state).await?;
    state
        .set_status("authenticated", "HydCraft 账户已恢复登录", None)
        .await;
    Ok(true)
}

#[tauri::command]
async fn logout_desktop(state: State<'_, UpdaterState>) -> Result<(), String> {
    if let Some(refresh_token) = load_desktop_refresh_token() {
        revoke_desktop_session(&state.console_origin, &refresh_token).await?;
    }
    clear_desktop_refresh_token();
    *state.access_token.write().await = None;
    *state.identity.write().await = None;
    state
        .set_status("unauthenticated", "HydCraft 账户已退出登录", None)
        .await;
    Ok(())
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopAuthEvent {
    phase: String,
    message: String,
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

fn get_or_create_auth_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    if let Some(window) = app.get_webview_window("auth") {
        return Ok(window);
    }
    WebviewWindowBuilder::new(app, "auth", WebviewUrl::App("index.html".into()))
        .title("HydCraft 账户验证")
        .inner_size(420.0, 500.0)
        .center()
        .resizable(false)
        .decorations(false)
        .visible(false)
        .build()
        .map_err(|error| error.to_string())
}

async fn process_deep_links(app: AppHandle, state: UpdaterState, urls: Vec<Url>) {
    for url in urls {
        let result = async {
            let code = parse_desktop_callback(&url)?;
            exchange_desktop_code_value(code, state.clone()).await?;
            Ok::<(), String>(())
        }
        .await;

        match result {
            Ok(()) => {
                if let Ok(auth) = get_or_create_auth_window(&app) {
                    let _ = auth.show();
                    let _ = auth.set_focus();
                }
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
                if let Ok(auth) = get_or_create_auth_window(&app) {
                    let _ = auth.show();
                    let _ = auth.set_focus();
                }
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
fn start_desktop_login(app: AppHandle, state: State<'_, UpdaterState>) -> Result<(), String> {
    let window = get_or_create_auth_window(&app)?;
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())?;
    let url = desktop_login_url_value(&state.console_origin);
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

#[tauri::command]
fn hide_auth_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("auth") {
        window.hide().map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn open_external_url(app: AppHandle, url: String) -> Result<(), String> {
    let parsed = Url::parse(&url).map_err(|error| error.to_string())?;
    match parsed.scheme() {
        "http" | "https" => app
            .opener()
            .open_url(url, None::<String>)
            .map_err(|error| error.to_string()),
        _ => Err("Only http/https URLs are allowed".into()),
    }
}

async fn execute_update(
    _app: AppHandle,
    state: UpdaterState,
    mut selected_version: Option<String>,
    source_key: Option<String>,
) {
    loop {
        match engine::apply_next(&state, selected_version.take(), source_key.clone()).await {
            Ok(Some(version)) => {
                state
                    .set_status("updating", &format!("客户端正在更新至 {version}"), None)
                    .await;
            }
            Ok(None) => {
                state.set_status("ready", "客户端已准备就绪", None).await;
                break;
            }
            Err(EngineError::Conflicts(conflicts)) => {
                *state.conflicts.write().await = conflicts;
                state
                    .set_status("awaiting-conflict-resolution", "发现受管文件冲突", None)
                    .await;
                break;
            }
            Err(error) => {
                state
                    .set_status("failed", &format!("更新失败：{error}"), None)
                    .await;
                break;
            }
        }
    }
}

async fn initialize_updater(state: UpdaterState) {
    match engine::inspect_client_version(&state.game_dir) {
        Ok(Some(version)) => {
            *state.selected_version.write().await = Some(version.clone());
            match engine::check_next(&state, &version).await {
                Ok(check) if check.update_available => {
                    state
                        .set_status("awaiting-update-decision", "发现客户端更新", None)
                        .await
                }
                Ok(_) => {
                    state
                        .set_status("up-to-date", "客户端已是最新版本", None)
                        .await
                }
                Err(error) => {
                    state
                        .set_status("failed", &format!("检查客户端更新失败：{error}"), None)
                        .await
                }
            }
        }
        Ok(None) => {
            state
                .set_status("awaiting-version", "请选择当前客户端版本", None)
                .await
        }
        Err(error) => {
            state
                .set_status("failed", &format!("无法读取当前客户端版本：{error}"), None)
                .await
        }
    }
}

pub fn run() {
    let arguments = env::args().collect::<Vec<_>>();
    let game_dir = argument_value(&arguments, "--game-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().expect("current directory"));
    let console_origin = argument_value(&arguments, "--console-origin")
        .unwrap_or_else(|| "https://console.hydcraft.cn".into());
    let mode = argument_value(&arguments, "--mode").unwrap_or_else(|| "manual".into());
    let state = UpdaterState::new(game_dir, console_origin, mode);
    let update_state = state.clone();
    let callback_state = state.clone();
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.show();
                let _ = main.set_focus();
            }
        }));
    }

    builder
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .setup(move |app| {
            #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
            app.deep_link().register_all()?;

            let app_handle = app.handle().clone();
            let event_state = callback_state.clone();
            app.deep_link().on_open_url(move |event| {
                let app_handle = app_handle.clone();
                let state = event_state.clone();
                tauri::async_runtime::spawn(process_deep_links(
                    app_handle,
                    state,
                    event.urls().to_vec(),
                ));
            });

            if let Some(urls) = app.deep_link().get_current()? {
                tauri::async_runtime::spawn(process_deep_links(
                    app.handle().clone(),
                    callback_state.clone(),
                    urls,
                ));
            }

            let app_handle = app.handle().clone();
            let startup_state = update_state.clone();
            tauri::async_runtime::spawn(async move {
                startup_state.bind_app(app_handle).await;
                let _ = restore_desktop_session(startup_state.clone()).await;
                initialize_updater(startup_state).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            updater_status,
            updater_context,
            client_version_options,
            download_sources,
            inspect_client,
            open_version_window,
            hide_version_window,
            desktop_identity,
            pending_conflicts,
            resolve_conflicts,
            select_current_version,
            select_download_source,
            begin_update,
            recheck_update,
            skip_update,
            launch_client,
            desktop_login_url,
            exchange_desktop_code,
            logout_desktop,
            start_desktop_login,
            hide_auth_window,
            open_external_url
        ])
        .run(tauri::generate_context!())
        .expect("HydCraft Updater failed to run");
}

fn argument_value(arguments: &[String], name: &str) -> Option<String> {
    arguments
        .iter()
        .position(|argument| argument == name)
        .and_then(|index| arguments.get(index + 1))
        .cloned()
}
