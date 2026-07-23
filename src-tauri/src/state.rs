use crate::contracts::{UpdateConflict, UpdaterStatus};
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
                remaining_seconds: None,
                current_version: None,
                target_version: None,
            })),
            selected_version: Arc::new(RwLock::new(None)),
            selected_source: Arc::new(RwLock::new(None)),
            access_token: Arc::new(RwLock::new(None)),
            identity: Arc::new(RwLock::new(None)),
            conflicts: Arc::new(RwLock::new(Vec::new())),
            resolutions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    pub async fn set_status(&self, phase: &str, message: &str, remaining: Option<u8>) {
        let status = UpdaterStatus {
            mode: self.mode.clone(),
            phase: phase.into(),
            message: message.into(),
            remaining_seconds: remaining,
            current_version: None,
            target_version: None,
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
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopIdentity {
    pub hydroline_id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}
