mod network;
mod package;
mod storage;
mod transaction;

use crate::{contracts::MigrationEnvelope, state::UpdaterState};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("conflicts require user confirmation")]
    Conflicts(Vec<crate::contracts::UpdateConflict>),
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientUpdateCheck {
    pub current_version: String,
    pub update_available: bool,
    pub to_version: String,
    pub migration_id: Option<String>,
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
    pub priority: i32,
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

pub async fn apply_next(
    state: &UpdaterState,
    selected_version: Option<String>,
    source_key: Option<String>,
) -> Result<Option<String>, EngineError> {
    let mut client_state = storage::load_client_state(&state.game_dir)?;
    if client_state.current_version.is_empty() {
        client_state.current_version = selected_version
            .ok_or_else(|| EngineError::Message("请选择当前客户端版本后再更新".into()))?;
    }
    let migration =
        match fetch_next(state, &client_state.current_version, source_key.as_deref()).await? {
            Some(value) => value,
            None => return Ok(None),
        };
    package::verify_envelope(&migration)?;
    let anchors = if migration.anchors.is_empty() {
        &migration.plan.anchors
    } else {
        &migration.anchors
    };
    storage::verify_anchors(&state.game_dir, anchors)?;
    let bytes = network::download_package(state, &migration).await?;
    package::verify_package(&bytes, &migration)?;
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
        &state.game_dir,
        &extracted,
        &client_state,
        &resolutions,
        &bytes,
        anchors,
    )?;
    if !conflicts.is_empty() {
        return Err(EngineError::Conflicts(conflicts));
    }
    transaction::apply_transaction(
        &state.game_dir,
        &extracted,
        &bytes,
        &mut client_state,
        &resolutions,
    )?;
    Ok(Some(migration.to_version))
}
