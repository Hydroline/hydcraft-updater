use crate::contracts::{DownloadProgress, UpdateConflict, UpdaterStatus};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct UpdaterState {
    pub game_dir: PathBuf,
    pub console_origin: String,
    pub mode: String,
    pub app: Arc<RwLock<Option<AppHandle>>>,
    pub status: Arc<RwLock<UpdaterStatus>>,
    pub selected_version: Arc<RwLock<Option<String>>>,
    pub selected_source: Arc<RwLock<Option<String>>>,
    pub access_token: Arc<RwLock<Option<String>>>,
    pub identity: Arc<RwLock<Option<DesktopIdentity>>>,
    pub conflicts: Arc<RwLock<Vec<UpdateConflict>>>,
    pub resolutions: Arc<RwLock<HashMap<String, String>>>,
    pub client_details: Arc<RwLock<Option<ClientDetailsRequest>>>,
}

impl UpdaterState {
    pub fn new(game_dir: PathBuf, console_origin: String, mode: String) -> Self {
        Self {
            game_dir,
            console_origin,
            mode: mode.clone(),
            app: Arc::new(RwLock::new(None)),
            status: Arc::new(RwLock::new(UpdaterStatus {
                mode,
                phase: "checking-migration".into(),
                message: "正在获取客户端更新迁移".into(),
                failure_kind: None,
                remaining_seconds: None,
                current_version: None,
                target_version: None,
                download: None,
            })),
            selected_version: Arc::new(RwLock::new(None)),
            selected_source: Arc::new(RwLock::new(None)),
            access_token: Arc::new(RwLock::new(None)),
            identity: Arc::new(RwLock::new(None)),
            conflicts: Arc::new(RwLock::new(Vec::new())),
            resolutions: Arc::new(RwLock::new(HashMap::new())),
            client_details: Arc::new(RwLock::new(None)),
        }
    }
    pub async fn set_status(&self, phase: &str, message: &str, remaining: Option<u8>) {
        self.set_status_with_versions(phase, message, remaining, None, None)
            .await;
    }

    pub async fn set_status_with_versions(
        &self,
        phase: &str,
        message: &str,
        remaining: Option<u8>,
        current_version: Option<String>,
        target_version: Option<String>,
    ) {
        let status = UpdaterStatus {
            mode: self.mode.clone(),
            phase: phase.into(),
            message: message.into(),
            failure_kind: None,
            remaining_seconds: remaining,
            current_version,
            target_version,
            download: None,
        };
        *self.status.write().await = status.clone();
        if let Some(app) = self.app.read().await.clone() {
            let _ = app.emit("updater-status", status);
        }
    }

    pub async fn set_failure_status(&self, message: &str, failure_kind: &str) {
        let status = UpdaterStatus {
            mode: self.mode.clone(),
            phase: "failed".into(),
            message: message.into(),
            failure_kind: Some(failure_kind.into()),
            remaining_seconds: None,
            current_version: None,
            target_version: None,
            download: None,
        };
        *self.status.write().await = status.clone();
        if let Some(app) = self.app.read().await.clone() {
            let _ = app.emit("updater-status", status);
        }
    }

    pub async fn set_download_status(&self, message: &str, download: DownloadProgress) {
        let status = UpdaterStatus {
            mode: self.mode.clone(),
            phase: "updating".into(),
            message: message.into(),
            failure_kind: None,
            remaining_seconds: None,
            current_version: None,
            target_version: None,
            download: Some(download),
        };
        *self.status.write().await = status.clone();
        if let Some(app) = self.app.read().await.clone() {
            let _ = app.emit("updater-status", status);
        }
    }

    pub async fn bind_app(&self, app: AppHandle) {
        *self.app.write().await = Some(app);
    }
}

#[derive(Clone)]
pub struct ClientDetailsRequest {
    pub version: String,
    pub detail: String,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopIdentity {
    pub hydroline_id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}
