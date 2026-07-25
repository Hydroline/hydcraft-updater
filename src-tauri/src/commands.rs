use crate::{
    engine, lifecycle,
    state::{ClientDetailsRequest, DesktopIdentity, UpdaterState},
    windows,
};
use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use url::Url;

#[tauri::command]
pub async fn updater_status(
    state: State<'_, UpdaterState>,
) -> Result<crate::contracts::UpdaterStatus, String> {
    Ok(state.status.read().await.clone())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdaterContext {
    pub mode: String,
    pub game_dir: String,
    pub console_origin: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientInspection {
    pub version: Option<String>,
    pub needs_selection: bool,
}

#[tauri::command]
pub async fn updater_context(state: State<'_, UpdaterState>) -> Result<UpdaterContext, String> {
    Ok(UpdaterContext {
        mode: state.mode.clone(),
        game_dir: state.game_dir.to_string_lossy().into_owned(),
        console_origin: state.console_origin.clone(),
    })
}

#[tauri::command]
pub async fn client_version_options(
    state: State<'_, UpdaterState>,
) -> Result<Vec<engine::ClientVersionOption>, String> {
    engine::list_versions(&state)
        .await
        .map_err(|error| error.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientDetailsWindowData {
    pub detail: String,
    pub version: engine::ClientVersionOption,
}

#[tauri::command]
pub async fn open_client_details_window(
    version: String,
    detail: String,
    app: AppHandle,
    state: State<'_, UpdaterState>,
) -> Result<(), String> {
    if !matches!(detail.as_str(), "changelog" | "mods") {
        return Err("CLIENT_DETAIL_KIND_INVALID".into());
    }
    *state.client_details.write().await = Some(ClientDetailsRequest { version, detail });
    windows::open_client_details_window(app)
}

#[tauri::command]
pub async fn client_details_window_data(
    state: State<'_, UpdaterState>,
) -> Result<ClientDetailsWindowData, String> {
    let request = state
        .client_details
        .read()
        .await
        .clone()
        .ok_or_else(|| "CLIENT_DETAILS_NOT_SELECTED".to_string())?;
    let version = engine::list_versions(&state)
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|option| option.version == request.version)
        .ok_or_else(|| "CLIENT_VERSION_NOT_FOUND".to_string())?;
    Ok(ClientDetailsWindowData {
        detail: request.detail,
        version,
    })
}

#[tauri::command]
pub async fn download_sources(
    locale: Option<String>,
    state: State<'_, UpdaterState>,
) -> Result<Vec<engine::DownloadSource>, String> {
    engine::list_sources(&state, locale.as_deref().unwrap_or("zh-CN"))
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn inspect_client(state: State<'_, UpdaterState>) -> Result<ClientInspection, String> {
    match engine::inspect_client_version(&state.game_dir) {
        Ok(version) => Ok(ClientInspection {
            needs_selection: version.is_none(),
            version,
        }),
        Err(error) => Err(error.to_string()),
    }
}

#[tauri::command]
pub fn open_version_window(app: AppHandle) -> Result<(), String> {
    windows::open_version_window(app)
}

#[tauri::command]
pub fn hide_version_window(app: AppHandle) -> Result<(), String> {
    windows::hide_version_window(app)
}

#[tauri::command]
pub async fn desktop_identity(
    state: State<'_, UpdaterState>,
) -> Result<Option<DesktopIdentity>, String> {
    Ok(state.identity.read().await.clone())
}

#[tauri::command]
pub async fn pending_conflicts(
    state: State<'_, UpdaterState>,
) -> Result<Vec<crate::contracts::UpdateConflict>, String> {
    Ok(state.conflicts.read().await.clone())
}

#[tauri::command]
pub async fn resolve_conflicts(
    resolutions: std::collections::HashMap<String, String>,
    state: State<'_, UpdaterState>,
) -> Result<crate::contracts::UpdaterStatus, String> {
    *state.resolutions.write().await = resolutions;
    state.conflicts.write().await.clear();
    let selected = state.selected_version.read().await.clone();
    let source = state.selected_source.read().await.clone();
    state.set_status("updating", "正在更新客户端", None).await;
    lifecycle::spawn_update(state.inner().clone(), selected, source);
    Ok(state.status.read().await.clone())
}

#[tauri::command]
pub async fn select_current_version(
    version: String,
    state: State<'_, UpdaterState>,
) -> Result<crate::contracts::UpdaterStatus, String> {
    *state.selected_version.write().await = Some(version.clone());
    state
        .set_status("checking-update", "正在校验所选客户端版本", None)
        .await;
    match engine::check_next(&state, &version).await {
        Ok(check) if check.update_available => {
            state
                .set_status_with_versions(
                    "awaiting-update-decision",
                    "发现客户端更新",
                    None,
                    Some(check.current_version),
                    Some(check.to_version),
                )
                .await;
        }
        Ok(_) => {
            state
                .set_status("up-to-date", "客户端已是最新版本", None)
                .await
        }
        Err(error) => {
            state
                .set_failure_status(&format!("无法检查当前客户端版本：{error}"), "check")
                .await
        }
    }
    Ok(state.status.read().await.clone())
}

#[tauri::command]
pub async fn select_download_source(
    source_key: String,
    state: State<'_, UpdaterState>,
) -> Result<(), String> {
    *state.selected_source.write().await = Some(source_key);
    Ok(())
}

#[tauri::command]
pub async fn begin_update(state: State<'_, UpdaterState>) -> Result<(), String> {
    let selected = state.selected_version.read().await.clone();
    let source = state.selected_source.read().await.clone();
    state.set_status("updating", "正在更新客户端", None).await;
    lifecycle::spawn_update(state.inner().clone(), selected, source);
    Ok(())
}

#[tauri::command]
pub async fn recheck_update(state: State<'_, UpdaterState>) -> Result<(), String> {
    let Some(version) = state.selected_version.read().await.clone() else {
        return Ok(());
    };
    state
        .set_status("checking-update", "正在检查客户端更新", None)
        .await;
    match engine::check_next(&state, &version).await {
        Ok(check) if check.update_available => {
            state
                .set_status_with_versions(
                    "awaiting-update-decision",
                    "发现客户端更新",
                    None,
                    Some(check.current_version),
                    Some(check.to_version),
                )
                .await
        }
        Ok(_) => {
            state
                .set_status("up-to-date", "客户端已是最新版本", None)
                .await
        }
        Err(error) => {
            state
                .set_failure_status(&format!("更新检查失败：{error}"), "check")
                .await
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn launch_client(app: AppHandle, state: State<'_, UpdaterState>) -> Result<(), String> {
    if state.mode != "bootstrap" {
        return Err("BOOTSTRAP_REQUIRED".into());
    }
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn play_failure_sound() -> Result<(), String> {
    system_error_beep();
    Ok(())
}

#[cfg(target_os = "windows")]
fn system_error_beep() {
    const MB_ICONERROR: u32 = 0x00000010;

    #[link(name = "user32")]
    extern "system" {
        fn MessageBeep(u_type: u32) -> i32;
    }

    unsafe {
        let _ = MessageBeep(MB_ICONERROR);
    }
}

#[cfg(not(target_os = "windows"))]
fn system_error_beep() {}

#[tauri::command]
pub fn hide_auth_window(app: AppHandle) -> Result<(), String> {
    crate::windows::hide_auth_window(app)
}

#[tauri::command]
pub fn open_external_url(app: AppHandle, url: String) -> Result<(), String> {
    let parsed = Url::parse(&url).map_err(|error| error.to_string())?;
    match parsed.scheme() {
        "http" | "https" => app
            .opener()
            .open_url(url, None::<String>)
            .map_err(|error| error.to_string()),
        _ => Err("Only http/https URLs are allowed".into()),
    }
}
