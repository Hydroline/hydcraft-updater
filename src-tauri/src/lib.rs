mod addons;
mod contracts;
mod manifest;
mod mcpatch;
mod proxy;
mod state;

use contracts::{AddonCategory, UpdaterStatus};
use state::UpdaterState;
use std::{env, path::PathBuf};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use url::Url;

#[tauri::command]
async fn updater_status(state: State<'_, UpdaterState>) -> Result<UpdaterStatus, String> {
    Ok(state.status.read().await.clone())
}

#[tauri::command]
async fn hold_for_user_interaction(
    state: State<'_, UpdaterState>,
) -> Result<UpdaterStatus, String> {
    *state.user_interacted.write().await = true;
    state
        .set_status("awaiting-addon-selection", "正在等待选加包配置", None)
        .await;
    Ok(state.status.read().await.clone())
}

#[tauri::command]
fn desktop_login_url(state: State<'_, UpdaterState>) -> String {
    format!(
        "{}/api/oidc/hydcraft/login?desktop_redirect_uri=hydcraft-updater%3A%2F%2Fauth%2Fcallback",
        state.console_origin.trim_end_matches('/')
    )
}

#[tauri::command]
async fn exchange_desktop_code(
    code: String,
    state: State<'_, UpdaterState>,
) -> Result<UpdaterStatus, String> {
    let response = reqwest::Client::new()
        .post(format!(
            "{}/api/desktop-auth/exchange",
            state.console_origin.trim_end_matches('/')
        ))
        .json(&serde_json::json!({ "code": code }))
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "Desktop authorization failed: {}",
            response.status()
        ));
    }
    let token = response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| error.to_string())?
        .get("accessToken")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "Console returned no access token".to_string())?
        .to_owned();
    *state.access_token.write().await = Some(token);
    state
        .set_status(
            "authenticated",
            "HydCraft 账户已登录，正在刷新清单",
            Some(10),
        )
        .await;
    Ok(state.status.read().await.clone())
}

#[tauri::command]
async fn apply_addon_selection(
    ids: Vec<String>,
    state: State<'_, UpdaterState>,
    app: AppHandle,
) -> Result<UpdaterStatus, String> {
    let manifest = manifest::load(&state).await?;
    state
        .set_status("syncing-addons", "正在同步选加包", None)
        .await;
    addons::apply_selection(&state, &manifest, &ids).await?;
    state
        .set_status("ready", "选加包已同步，准备启动客户端", Some(10))
        .await;
    *state.user_interacted.write().await = false;
    tauri::async_runtime::spawn(exit_after_ready(app, state.inner().clone()));
    Ok(state.status.read().await.clone())
}

#[tauri::command]
async fn available_addons(state: State<'_, UpdaterState>) -> Result<Vec<AddonCategory>, String> {
    Ok(manifest::load(&state).await?.categories)
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

async fn execute_update(app: AppHandle, state: UpdaterState) {
    state
        .set_status("loading-manifest", "正在获取更新清单", Some(10))
        .await;
    let result = async {
        let manifest = manifest::load(&state).await?;
        state
            .set_status("preparing-download", "正在准备客户端下载源", Some(10))
            .await;
        let proxy_url = proxy::start(
            state.console_origin.clone(),
            manifest.client_version.clone(),
            state.access_token.read().await.clone(),
        )
        .await?;
        state
            .set_status(
                "syncing-client",
                "正在通过 MCPatch 同步基础客户端",
                Some(10),
            )
            .await;
        mcpatch::update_client(&state, &manifest, &proxy_url).await
    }
    .await;
    if let Err(error) = result {
        state
            .set_status("failed", &format!("更新失败：{error}"), None)
            .await;
        return;
    }
    state
        .set_status("ready", "基础客户端已就绪", Some(10))
        .await;
    exit_after_ready(app, state).await;
}

async fn exit_after_ready(app: AppHandle, state: UpdaterState) {
    for remaining in (1..=10).rev() {
        if *state.user_interacted.read().await {
            return;
        }
        state
            .set_status("ready", "基础客户端已就绪", Some(remaining))
            .await;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    app.exit(0);
}

pub fn run() {
    let arguments = env::args().collect::<Vec<_>>();
    let game_dir = argument_value(&arguments, "--game-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().expect("current directory"));
    let console_origin = argument_value(&arguments, "--console-origin")
        .unwrap_or_else(|| "https://console.hydcraft.cn".into());
    let state = UpdaterState::new(game_dir, console_origin);
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state.clone())
        .setup(move |app| {
            tauri::async_runtime::spawn(execute_update(app.handle().clone(), state));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            updater_status,
            hold_for_user_interaction,
            desktop_login_url,
            exchange_desktop_code,
            apply_addon_selection,
            available_addons,
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
