use crate::{
    build_info,
    contracts::UpdaterStatus,
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
    pub updater_version: String,
    pub updater_commit_sha: String,
    pub updater_platform: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientInspection {
    pub version: Option<String>,
    pub needs_selection: bool,
}

#[tauri::command]
pub async fn updater_context(state: State<'_, UpdaterState>) -> Result<UpdaterContext, String> {
    let build = build_info::current();
    Ok(UpdaterContext {
        mode: state.mode.clone(),
        game_dir: state.game_dir.to_string_lossy().into_owned(),
        console_origin: state.console_origin.clone(),
        updater_version: build.version.into(),
        updater_commit_sha: build.commit_sha.into(),
        updater_platform: build.platform.into(),
    })
}

#[tauri::command]
pub fn client_storage_info(
    state: State<'_, UpdaterState>,
) -> Result<engine::ClientStorageInfo, String> {
    engine::storage_info(&state.game_dir).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn clean_downloads(
    state: State<'_, UpdaterState>,
) -> Result<engine::ClientStorageInfo, String> {
    if state.status.read().await.phase == "updating" {
        return Err("更新正在进行，请完成更新后再清理缓存".into());
    }
    engine::clean_downloads(&state.game_dir).map_err(|error| error.to_string())?;
    engine::storage_info(&state.game_dir).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn clean_backups(
    state: State<'_, UpdaterState>,
) -> Result<engine::ClientStorageInfo, String> {
    if state.status.read().await.phase == "updating" {
        return Err("更新正在进行，请完成更新后再清理回滚备份".into());
    }
    engine::clean_backups(&state.game_dir).map_err(|error| error.to_string())?;
    engine::storage_info(&state.game_dir).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn rollback_last_update(state: State<'_, UpdaterState>) -> Result<(), String> {
    if state.status.read().await.phase == "updating" {
        return Err("更新正在进行，请完成更新后再回滚".into());
    }
    crate::logging::append(&state.game_dir, "START", "Manual rollback requested");
    if let Err(error) = engine::rollback_last_update(&state.game_dir) {
        crate::logging::append(
            &state.game_dir,
            "ERROR",
            format!("Manual rollback failed: {error}"),
        );
        return Err(error.to_string());
    }
    crate::logging::append(&state.game_dir, "SUCCESS", "Manual rollback finished");
    state
        .set_status("checking-update", "正在检查回滚后的客户端状态", None)
        .await;
    Ok(())
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
pub async fn cancel_conflict_resolution(
    state: State<'_, UpdaterState>,
) -> Result<crate::contracts::UpdaterStatus, String> {
    state.conflicts.write().await.clear();
    state.resolutions.write().await.clear();
    let current_version = state.selected_version.read().await.clone();
    let Some(current_version) = current_version else {
        state
            .set_status("awaiting-version", "需要确认客户端版本", None)
            .await;
        return Ok(state.status.read().await.clone());
    };
    state
        .set_status("checking-update", "正在检查客户端更新", None)
        .await;
    match engine::check_next(&state, &current_version).await {
        Ok(check) => {
            lifecycle::present_update_check(state.inner().clone(), check).await;
        }
        Err(error) => {
            state
                .set_failure_status(&format!("无法重新检查客户端更新：{error}"), "check")
                .await;
        }
    }
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
        Ok(check) => lifecycle::present_update_check(state.inner().clone(), check).await,
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
pub async fn begin_update(
    clean_downloads_after_install: bool,
    source_key: Option<String>,
    state: State<'_, UpdaterState>,
) -> Result<(), String> {
    *state.clean_downloads_after_install.write().await = clean_downloads_after_install;
    if let Some(source_key) = source_key.filter(|value| !value.trim().is_empty()) {
        *state.selected_source.write().await = Some(source_key);
    }
    state.disarm_bootstrap_auto_countdown().await;
    let selected = state.selected_version.read().await.clone();
    let source = state.selected_source.read().await.clone();
    state.set_status("updating", "正在更新客户端", None).await;
    lifecycle::spawn_update(state.inner().clone(), selected, source);
    Ok(())
}

#[tauri::command]
pub async fn cancel_bootstrap_auto_countdown(
    state: State<'_, UpdaterState>,
) -> Result<UpdaterStatus, String> {
    let had_countdown = state.status.read().await.remaining_seconds.is_some();
    let status = state.cancel_bootstrap_auto_countdown().await;
    if had_countdown && status.remaining_seconds.is_none() {
        crate::logging::append(
            &state.game_dir,
            "INFO",
            "Bootstrap automatic action cancelled after explicit pointer movement",
        );
    }
    Ok(status)
}

#[tauri::command]
pub async fn install_client_version(
    version: String,
    mode: String,
    clean_downloads_after_install: bool,
    state: State<'_, UpdaterState>,
) -> Result<(), String> {
    *state.clean_downloads_after_install.write().await = clean_downloads_after_install;
    state
        .set_status_with_versions(
            "updating",
            "正在下载完整客户端包",
            None,
            None,
            Some(version.clone()),
        )
        .await;
    lifecycle::spawn_client_install(state.inner().clone(), version, mode);
    Ok(())
}

#[tauri::command]
pub async fn recheck_update(state: State<'_, UpdaterState>) -> Result<(), String> {
    let inspected_version = engine::inspect_client_version(&state.game_dir)
        .ok()
        .flatten();
    let version = match inspected_version {
        Some(version) => Some(version),
        None => state.selected_version.read().await.clone(),
    };
    let Some(version) = version else {
        return Ok(());
    };
    *state.selected_version.write().await = Some(version.clone());
    state
        .set_status("checking-update", "正在检查客户端更新", None)
        .await;
    match engine::check_next(&state, &version).await {
        Ok(check) => lifecycle::present_update_check(state.inner().clone(), check).await,
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
    let phase = state.status.read().await.phase.clone();
    if !matches!(phase.as_str(), "ready" | "up-to-date") {
        return Err("UPDATER_NOT_READY".into());
    }
    crate::logging::append(
        &state.game_dir,
        "SUCCESS",
        format!("Bootstrap launch permitted; phase={phase}"),
    );
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
