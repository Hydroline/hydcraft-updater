mod network;
mod package;
mod storage;
mod transaction;

use crate::{contracts::MigrationEnvelope, state::UpdaterState};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("conflicts require user confirmation")]
    Conflicts(Vec<crate::contracts::UpdateConflict>),
    #[error("{0}")]
    Message(String),
}

pub enum ApplyResult {
    Updated(String),
    PartiallyApplied,
    UpToDate,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientStorageInfo {
    pub downloads_bytes: u64,
    pub backups_bytes: u64,
    pub rollback_available: bool,
    pub rollback_from_version: Option<String>,
    pub rollback_to_version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientUpdateCheck {
    pub current_version: String,
    pub update_available: bool,
    pub to_version: String,
    pub migration_id: Option<String>,
    #[serde(default)]
    pub test_revision: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientVersionOption {
    pub version: String,
    pub label: String,
    pub is_latest: bool,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub changelog: Option<String>,
    #[serde(default)]
    pub api_version: Option<String>,
    #[serde(default)]
    pub mod_count: u32,
    #[serde(default)]
    pub mods: Vec<ClientMod>,
    #[serde(default)]
    pub is_base: bool,
    #[serde(default)]
    pub publisher: Option<ClientReleasePerson>,
    #[serde(default)]
    pub contributors: Vec<ClientReleasePerson>,
    #[serde(default)]
    pub full_package: Option<ClientFullPackage>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientReleasePerson {
    pub hydroline_id: String,
    pub username: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientFullPackage {
    pub package_key: String,
    pub package_sha256: String,
    pub package_size: u64,
    pub signature: String,
    #[serde(default)]
    pub signature_payload: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientFullPackageDownload {
    pub package_key: String,
    pub package_sha256: String,
    pub package_size: u64,
    pub signature: String,
    #[serde(default)]
    pub signature_payload: Option<String>,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientMod {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub api: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadSource {
    pub key: String,
    pub label: String,
    pub base_url: String,
    pub priority: i32,
    #[serde(default)]
    pub is_default: bool,
    pub requires_login: bool,
    pub available: bool,
    #[serde(default)]
    pub latency_ms: Option<u32>,
}

pub async fn fetch_next(
    state: &UpdaterState,
    version: &str,
    source_key: Option<&str>,
) -> Result<Option<MigrationEnvelope>, EngineError> {
    network::fetch_next(state, version, source_key).await
}

pub async fn check_next(
    state: &UpdaterState,
    version: &str,
) -> Result<ClientUpdateCheck, EngineError> {
    network::check_next(state, version).await
}

pub async fn list_versions(state: &UpdaterState) -> Result<Vec<ClientVersionOption>, EngineError> {
    network::list_versions(state).await
}

pub async fn list_sources(
    state: &UpdaterState,
    locale: &str,
) -> Result<Vec<DownloadSource>, EngineError> {
    network::list_sources(state, locale).await
}

pub fn inspect_client_version(game: &std::path::Path) -> Result<Option<String>, EngineError> {
    storage::inspect_client_version(game)
}

pub fn recover_unfinished_transaction(game: &std::path::Path) -> Result<bool, EngineError> {
    transaction::recover_unfinished_transaction(game)
}

pub fn storage_info(game: &Path) -> Result<ClientStorageInfo, EngineError> {
    let state = storage::load_client_state(game)?;
    let rollback = state.last_transaction.filter(|transaction| {
        storage::transaction_backup_path(game, &transaction.migration_id)
            .map(|path| path.is_dir())
            .unwrap_or(false)
    });
    Ok(ClientStorageInfo {
        downloads_bytes: storage::directory_size(&storage::downloads_path(game))?,
        backups_bytes: storage::directory_size(&storage::backups_path(game))?,
        rollback_available: rollback.is_some(),
        rollback_from_version: rollback.as_ref().map(|value| value.from_version.clone()),
        rollback_to_version: rollback.as_ref().map(|value| value.to_version.clone()),
    })
}

pub fn clean_downloads(game: &Path) -> Result<(), EngineError> {
    storage::clear_directory(&storage::downloads_path(game))
}

pub fn clean_backups(game: &Path) -> Result<(), EngineError> {
    let mut state = storage::load_client_state(game)?;
    if state.unfinished_transaction.is_some() {
        return Err(EngineError::Message(
            "当前仍有未完成的更新，暂时不能清理回滚备份".into(),
        ));
    }
    storage::clear_directory(&storage::backups_path(game))?;
    state.last_transaction = None;
    storage::save_state(game, &state)
}

pub fn rollback_last_update(game: &Path) -> Result<(), EngineError> {
    transaction::rollback_last_update(game)
}

pub async fn apply_next(
    state: &UpdaterState,
    selected_version: Option<String>,
    source_key: Option<String>,
) -> Result<ApplyResult, EngineError> {
    let mut client_state = storage::load_client_state(&state.game_dir)?;
    if client_state.current_version.is_empty() {
        client_state.current_version = selected_version
            .ok_or_else(|| EngineError::Message("请选择当前客户端版本后再更新".into()))?;
    }
    let migration =
        match fetch_next(state, &client_state.current_version, source_key.as_deref()).await? {
            Some(value) => value,
            None => return Ok(ApplyResult::UpToDate),
        };
    package::verify_envelope(&migration)?;
    let anchors = if migration.anchors.is_empty() {
        &migration.plan.anchors
    } else {
        &migration.anchors
    };
    let anchor_mismatches = storage::mismatched_anchors(&state.game_dir, anchors)?;
    let bytes = network::download_package(state, &migration).await?;
    package::verify_package(state, &bytes, &migration).await?;
    let console_plan = package::plan_from_envelope(&migration);
    // Console 迁移记录是执行计划的权威来源，ZIP 内计划只用于兼容性校验。
    if let Ok(value) = package::extract_plan(&bytes) {
        if value.migration_id != console_plan.migration_id
            || value.from_version != migration.from_version
            || value.to_version != migration.to_version
        {
            return Err(EngineError::Message(
                "ZIP 内更新计划与 Console 迁移记录不一致".into(),
            ));
        }
    }
    let extracted = console_plan;
    let resolutions = state.resolutions.read().await.clone();
    let conflicts = transaction::preflight_conflicts(
        state,
        &state.game_dir,
        &extracted,
        &client_state,
        &resolutions,
        &bytes,
        anchors,
        &anchor_mismatches,
    )
    .await?;
    if !conflicts.is_empty() {
        return Err(EngineError::Conflicts(conflicts));
    }
    let partially_applied = transaction::apply_transaction(
        state,
        &state.game_dir,
        &extracted,
        &bytes,
        &mut client_state,
        &resolutions,
    )
    .await?;
    if partially_applied {
        Ok(ApplyResult::PartiallyApplied)
    } else {
        Ok(ApplyResult::Updated(migration.to_version))
    }
}

pub async fn install_client_version(
    state: &UpdaterState,
    version: &str,
    mode: &str,
) -> Result<(), EngineError> {
    if !matches!(mode, "full" | "mods") {
        return Err(EngineError::Message("客户端覆盖模式无效".into()));
    }
    let release = list_versions(state)
        .await?
        .into_iter()
        .find(|release| release.version == version)
        .ok_or_else(|| EngineError::Message("客户端版本不存在".into()))?;
    let full_package = release
        .full_package
        .ok_or_else(|| EngineError::Message("此客户端版本没有可用完整包".into()))?;
    if !release.is_latest && !full_package.package_key.contains("/base/") {
        return Err(EngineError::Message("此客户端版本不支持完整包覆盖".into()));
    }
    let source_key = state.selected_source.read().await.clone();
    let package = network::fetch_full_package(state, version, source_key.as_deref()).await?;
    let bytes = network::download_full_package(state, &package).await?;
    state.set_operation_status("verifying", None, None).await;
    transaction::install_base_package(state, &bytes, &release.version, mode).await
}
