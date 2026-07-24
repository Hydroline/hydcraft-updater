use super::EngineError;
use crate::{contracts::MigrationEnvelope, state::UpdaterState};

pub(super) async fn fetch_next(
    state: &UpdaterState,
    version: &str,
    source_key: Option<&str>,
) -> Result<Option<MigrationEnvelope>, EngineError> {
    let mut request = reqwest::Client::new()
        .get(format!(
            "{}/api/updater/migrations/next",
            state.console_origin.trim_end_matches('/')
        ))
        .query(&[("currentVersion", version)]);
    if let Some(source_key) = source_key {
        request = request.query(&[("sourceKey", source_key)]);
    }
    if let Some(token) = state.access_token.read().await.clone() {
        request = request.bearer_auth(token);
    }
    let response = request
        .send()
        .await
        .map_err(|error| EngineError::Message(error.to_string()))?;
    if response.status().as_u16() == 204 {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err(EngineError::Message(format!(
            "Migration request failed: {}",
            response.status()
        )));
    }
    response
        .json()
        .await
        .map(Some)
        .map_err(|error| EngineError::Message(error.to_string()))
}

pub(super) async fn check_next(
    state: &UpdaterState,
    version: &str,
) -> Result<super::ClientUpdateCheck, EngineError> {
    let mut request = reqwest::Client::new()
        .get(format!(
            "{}/api/updater/check",
            state.console_origin.trim_end_matches('/')
        ))
        .query(&[("currentVersion", version)]);
    if let Some(token) = state.access_token.read().await.clone() {
        request = request.bearer_auth(token);
    }
    request
        .send()
        .await
        .map_err(|error| EngineError::Message(error.to_string()))?
        .error_for_status()
        .map_err(|error| EngineError::Message(error.to_string()))?
        .json()
        .await
        .map_err(|error| EngineError::Message(error.to_string()))
}

pub(super) async fn list_versions(
    state: &UpdaterState,
) -> Result<Vec<super::ClientVersionOption>, EngineError> {
    reqwest::Client::new()
        .get(format!(
            "{}/api/updater/versions",
            state.console_origin.trim_end_matches('/')
        ))
        .send()
        .await
        .map_err(|error| EngineError::Message(error.to_string()))?
        .error_for_status()
        .map_err(|error| EngineError::Message(error.to_string()))?
        .json()
        .await
        .map_err(|error| EngineError::Message(error.to_string()))
}

pub(super) async fn list_sources(
    state: &UpdaterState,
    locale: &str,
) -> Result<Vec<super::DownloadSource>, EngineError> {
    let mut request = reqwest::Client::new().get(format!(
        "{}/api/updater/sources?locale={locale}",
        state.console_origin.trim_end_matches('/')
    ));
    if let Some(token) = state.access_token.read().await.clone() {
        request = request.bearer_auth(token);
    }
    request
        .send()
        .await
        .map_err(|error| EngineError::Message(error.to_string()))?
        .error_for_status()
        .map_err(|error| EngineError::Message(error.to_string()))?
        .json()
        .await
        .map_err(|error| EngineError::Message(error.to_string()))
}

pub(super) async fn download_package(value: &MigrationEnvelope) -> Result<Vec<u8>, EngineError> {
    let client = reqwest::Client::new();
    let mut failure = String::new();
    for url in &value.package_urls {
        match client.get(url).send().await {
            Ok(response) if response.status().is_success() => {
                return response
                    .bytes()
                    .await
                    .map(|value| value.to_vec())
                    .map_err(|error| EngineError::Message(error.to_string()));
            }
            Ok(response) => failure = format!("{url}: {}", response.status()),
            Err(error) => failure = format!("{url}: {error}"),
        }
    }
    Err(EngineError::Message(format!("所有下载源均失败：{failure}")))
}
