use crate::contracts::UpdaterStatus;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct UpdaterState {
    pub game_dir: PathBuf,
    pub console_origin: String,
    pub status: Arc<RwLock<UpdaterStatus>>,
    pub user_interacted: Arc<RwLock<bool>>,
    pub access_token: Arc<RwLock<Option<String>>>,
}

impl UpdaterState {
    pub fn new(game_dir: PathBuf, console_origin: String) -> Self {
        Self {
            game_dir,
            console_origin,
            status: Arc::new(RwLock::new(UpdaterStatus {
                phase: "initializing".into(),
                message: "正在初始化 HydCraft Updater".into(),
                remaining_seconds: Some(10),
            })),
            user_interacted: Arc::new(RwLock::new(false)),
            access_token: Arc::new(RwLock::new(None)),
        }
    }
    pub async fn set_status(&self, phase: &str, message: &str, remaining: Option<u8>) {
        *self.status.write().await = UpdaterStatus {
            phase: phase.into(),
            message: message.into(),
            remaining_seconds: remaining,
        };
    }
}
