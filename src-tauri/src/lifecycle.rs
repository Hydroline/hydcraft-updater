use crate::{engine, engine::EngineError, state::UpdaterState};

pub async fn execute_update(
    state: UpdaterState,
    mut selected_version: Option<String>,
    source_key: Option<String>,
) {
    loop {
        match engine::apply_next(&state, selected_version.take(), source_key.clone()).await {
            Ok(Some(version)) => {
                state
                    .set_status("updating", &format!("客户端正在更新至 {version}"), None)
                    .await;
            }
            Ok(None) => {
                state.set_status("ready", "客户端已准备就绪", None).await;
                break;
            }
            Err(EngineError::Conflicts(conflicts)) => {
                *state.conflicts.write().await = conflicts;
                state
                    .set_status("awaiting-conflict-resolution", "发现受管文件冲突", None)
                    .await;
                break;
            }
            Err(error) => {
                state
                    .set_status("failed", &format!("更新失败：{error}"), None)
                    .await;
                break;
            }
        }
    }
}

pub async fn initialize_updater(state: UpdaterState) {
    match engine::inspect_client_version(&state.game_dir) {
        Ok(Some(version)) => {
            *state.selected_version.write().await = Some(version.clone());
            match engine::check_next(&state, &version).await {
                Ok(check) if check.update_available => {
                    state
                        .set_status("awaiting-update-decision", "发现客户端更新", None)
                        .await
                }
                Ok(_) => {
                    state
                        .set_status("up-to-date", "客户端已是最新版本", None)
                        .await
                }
                Err(error) => {
                    state
                        .set_status("failed", &format!("检查客户端更新失败：{error}"), None)
                        .await
                }
            }
        }
        Ok(None) => {
            state
                .set_status("awaiting-version", "请选择当前客户端版本", None)
                .await;
        }
        Err(error) => {
            state
                .set_status("failed", &format!("无法读取当前客户端版本：{error}"), None)
                .await;
        }
    }
}

pub fn spawn_update(
    state: UpdaterState,
    selected_version: Option<String>,
    source_key: Option<String>,
) {
    tauri::async_runtime::spawn(execute_update(state, selected_version, source_key));
}
