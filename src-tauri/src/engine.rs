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
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadSource {
    pub key: String,
    pub label: String,
    pub priority: i32,
    pub requires_login: bool,
    pub available: bool,
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

pub fn fallback_version_options() -> Vec<ClientVersionOption> {
    vec![ClientVersionOption {
        version: "__no-version__".into(),
        label: "__no-version__".into(),
        is_latest: false,
    }]
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
    storage::verify_anchors(&state.game_dir, &migration.anchors)?;
    let bytes = network::download_package(&migration).await?;
    package::verify_package(&bytes, &migration)?;
    let extracted = package::extract_plan(&bytes)?;
    if extracted.migration_id != migration.migration_id
        || extracted.from_version != migration.from_version
        || extracted.to_version != migration.to_version
    {
        return Err(EngineError::Message(
            "ZIP 内更新计划与 Console 迁移记录不一致".into(),
        ));
    }
    let resolutions = state.resolutions.read().await.clone();
    let conflicts = transaction::preflight_conflicts(
        &state.game_dir,
        &extracted,
        &client_state,
        &resolutions,
        &bytes,
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
