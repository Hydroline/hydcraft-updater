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
                    .set_failure_status(&format!("更新失败：{error}"), "update")
                    .await;
                break;
            }
        }
    }
}

pub async fn initialize_updater(state: UpdaterState) {
    if let Err(error) = engine::recover_unfinished_transaction(&state.game_dir) {
        state
            .set_failure_status(&format!("恢复未完成更新失败：{error}"), "update")
            .await;
        return;
    }
    match engine::inspect_client_version(&state.game_dir) {
        Ok(Some(version)) => {
            *state.selected_version.write().await = Some(version.clone());
            match engine::check_next(&state, &version).await {
                Ok(check) if check.update_available => {
                    state
                        .set_status_with_versions(
                            "awaiting-update-decision",
                            "发现客户端更新",
                            None,
                            Some(check.current_version),
                            Some(check.to_version),
                        )
                        .await
                }
                Ok(_) => {
                    state
                        .set_status("up-to-date", "客户端已是最新版本", None)
                        .await
                }
                Err(error) => {
                    state
                        .set_failure_status(&format!("检查客户端更新失败：{error}"), "check")
                        .await
                }
            }
        }
        Ok(None) => match engine::list_versions(&state).await {
            Ok(options) if options.is_empty() => {
                state.set_status("unknown-client", "", None).await;
            }
            Ok(_) => {
                state.set_status("awaiting-version", "", None).await;
            }
            Err(error) => {
                state.set_failure_status(&error.to_string(), "check").await;
            }
        },
        Err(error) => {
            state
                .set_failure_status(&format!("无法读取当前客户端版本：{error}"), "check")
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

pub async fn execute_client_install(state: UpdaterState, version: String, mode: String) {
    match engine::install_client_version(&state, &version, &mode).await {
        Ok(()) => {
            *state.selected_version.write().await = Some(version.clone());
            state
                .set_status_with_versions(
                    "ready",
                    "客户端完整包已覆盖完成",
                    None,
                    Some(version.clone()),
                    Some(version),
                )
                .await;
        }
        Err(error) => {
            state
                .set_failure_status(&format!("客户端完整包覆盖失败：{error}"), "update")
                .await;
        }
    }
}

pub fn spawn_client_install(state: UpdaterState, version: String, mode: String) {
    tauri::async_runtime::spawn(execute_client_install(state, version, mode));
}
