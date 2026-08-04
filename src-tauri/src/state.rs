use crate::contracts::{DownloadProgress, OperationProgress, UpdateConflict, UpdaterStatus};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc, time::Instant};
use tauri::{AppHandle, Emitter};
use tokio::sync::{Mutex, RwLock};

#[derive(Clone)]
pub struct UpdaterState {
    pub game_dir: PathBuf,
    pub console_origin: String,
    pub mode: String,
    pub app: Arc<RwLock<Option<AppHandle>>>,
    pub status: Arc<RwLock<UpdaterStatus>>,
    pub selected_version: Arc<RwLock<Option<String>>>,
    pub selected_source: Arc<RwLock<Option<String>>>,
    pub clean_downloads_after_install: Arc<RwLock<bool>>,
    pub access_token: Arc<RwLock<Option<String>>>,
    pub access_token_expires_at: Arc<RwLock<Option<Instant>>>,
    pub auth_refresh_lock: Arc<Mutex<()>>,
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
                test_revision: None,
                download: None,
                operation: None,
            })),
            selected_version: Arc::new(RwLock::new(None)),
            selected_source: Arc::new(RwLock::new(None)),
            clean_downloads_after_install: Arc::new(RwLock::new(false)),
            access_token: Arc::new(RwLock::new(None)),
            access_token_expires_at: Arc::new(RwLock::new(None)),
            auth_refresh_lock: Arc::new(Mutex::new(())),
            identity: Arc::new(RwLock::new(None)),
            conflicts: Arc::new(RwLock::new(Vec::new())),
            resolutions: Arc::new(RwLock::new(HashMap::new())),
            client_details: Arc::new(RwLock::new(None)),
        }
    }
    pub async fn set_status(&self, phase: &str, message: &str, remaining: Option<u8>) {
        let (current_version, target_version, test_revision) =
            self.preserved_update_context(phase).await;
        self.replace_status(UpdaterStatus {
            mode: self.mode.clone(),
            phase: phase.into(),
            message: message.into(),
            failure_kind: None,
            remaining_seconds: remaining,
            current_version,
            target_version,
            test_revision,
            download: None,
            operation: None,
        })
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
        self.replace_status(UpdaterStatus {
            mode: self.mode.clone(),
            phase: phase.into(),
            message: message.into(),
            failure_kind: None,
            remaining_seconds: remaining,
            current_version,
            target_version,
            test_revision: None,
            download: None,
            operation: None,
        })
        .await;
    }

    pub async fn set_status_with_update(
        &self,
        phase: &str,
        message: &str,
        current_version: String,
        target_version: String,
        test_revision: Option<u32>,
    ) {
        self.replace_status(UpdaterStatus {
            mode: self.mode.clone(),
            phase: phase.into(),
            message: message.into(),
            failure_kind: None,
            remaining_seconds: None,
            current_version: Some(current_version),
            target_version: Some(target_version),
            test_revision,
            download: None,
            operation: None,
        })
        .await;
    }

    pub async fn set_failure_status(&self, message: &str, failure_kind: &str) {
        let (current_version, target_version, test_revision) =
            self.preserved_update_context("failed").await;
        self.replace_status(UpdaterStatus {
            mode: self.mode.clone(),
            phase: "failed".into(),
            message: message.into(),
            failure_kind: Some(failure_kind.into()),
            remaining_seconds: None,
            current_version,
            target_version,
            test_revision,
            download: None,
            operation: None,
        })
        .await;
    }

    pub async fn set_download_status(&self, message: &str, download: DownloadProgress) {
        let (current_version, target_version, test_revision) =
            self.preserved_update_context("updating").await;
        self.replace_status(UpdaterStatus {
            mode: self.mode.clone(),
            phase: "updating".into(),
            message: message.into(),
            failure_kind: None,
            remaining_seconds: None,
            current_version,
            target_version,
            test_revision,
            download: Some(download),
            operation: None,
        })
        .await;
    }

    pub async fn set_operation_status(
        &self,
        stage: &str,
        completed_items: Option<u64>,
        total_items: Option<u64>,
    ) {
        let (current_version, target_version, test_revision) =
            self.preserved_update_context("updating").await;
        self.replace_status(UpdaterStatus {
            mode: self.mode.clone(),
            phase: "updating".into(),
            message: stage.into(),
            failure_kind: None,
            remaining_seconds: None,
            current_version,
            target_version,
            test_revision,
            download: None,
            operation: Some(OperationProgress {
                stage: stage.into(),
                completed_items,
                total_items,
            }),
        })
        .await;
    }

    async fn preserved_update_context(
        &self,
        next_phase: &str,
    ) -> (Option<String>, Option<String>, Option<u32>) {
        if matches!(
            next_phase,
            "awaiting-version" | "unknown-client" | "up-to-date" | "ready"
        ) {
            return (None, None, None);
        }
        let previous = self.status.read().await;
        if previous.current_version.is_some()
            && previous.target_version.is_some()
            && previous.current_version != previous.target_version
        {
            return (
                previous.current_version.clone(),
                previous.target_version.clone(),
                previous.test_revision,
            );
        }
        (None, None, None)
    }

    async fn replace_status(&self, status: UpdaterStatus) {
        *self.status.write().await = status.clone();
        if let Some(app) = self.app.read().await.clone() {
            let _ = app.emit("updater-status", status);
        }
    }

    pub async fn bind_app(&self, app: AppHandle) {
        *self.app.write().await = Some(app);
    }

    pub async fn exit_process(&self, code: i32) {
        if let Some(app) = self.app.read().await.clone() {
            app.exit(code);
        }
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
