use crate::{engine, engine::EngineError, state::UpdaterState};
use std::time::Duration;

const BOOTSTRAP_UP_TO_DATE_DISPLAY_DURATION: Duration = Duration::from_millis(1200);

async fn exit_bootstrap_after_up_to_date_display(state: &UpdaterState) {
    crate::logging::append(
        &state.game_dir,
        "INFO",
        "Bootstrap client is up to date; keeping status visible for 1200ms",
    );
    tokio::time::sleep(BOOTSTRAP_UP_TO_DATE_DISPLAY_DURATION).await;
    crate::logging::append(
        &state.game_dir,
        "SUCCESS",
        "Bootstrap up-to-date confirmation completed; exiting updater",
    );
    state.exit_process(0).await;
}

pub async fn execute_update(
    state: UpdaterState,
    mut selected_version: Option<String>,
    source_key: Option<String>,
) {
    crate::logging::append(&state.game_dir, "START", "Update execution started");
    let clean_downloads_after_install = *state.clean_downloads_after_install.read().await;
    let mut updated_version = None;
    loop {
        match engine::apply_next(&state, selected_version.take(), source_key.clone()).await {
            Ok(engine::ApplyResult::Updated(version)) => {
                crate::logging::append(
                    &state.game_dir,
                    "INFO",
                    format!("Applied update to {version}"),
                );
                updated_version = Some(version.clone());
                state
                    .set_status("updating", &format!("客户端正在更新至 {version}"), None)
                    .await;
            }
            Ok(engine::ApplyResult::PartiallyApplied) => {
                crate::logging::append(
                    &state.game_dir,
                    "ERROR",
                    "Update stopped after a partial transaction",
                );
                state.set_status("partial-update", "", None).await;
                break;
            }
            Ok(engine::ApplyResult::UpToDate) => {
                crate::logging::append(
                    &state.game_dir,
                    "SUCCESS",
                    "Update execution finished: up to date",
                );
                if updated_version.is_some() && clean_downloads_after_install {
                    let _ = engine::clean_downloads(&state.game_dir);
                }
                if let Some(version) = updated_version {
                    state
                        .set_status_with_versions(
                            "ready",
                            "客户端已准备就绪",
                            None,
                            Some(version.clone()),
                            Some(version),
                        )
                        .await;
                } else {
                    state
                        .set_status("up-to-date", "客户端已是最新版本", None)
                        .await;
                }
                if state.mode == "bootstrap" {
                    crate::logging::append(
                        &state.game_dir,
                        "INFO",
                        "Bootstrap update finished; exiting updater",
                    );
                    state.exit_process(0).await;
                }
                break;
            }
            Err(EngineError::Conflicts(conflicts)) => {
                crate::logging::append(
                    &state.game_dir,
                    "WARN",
                    format!("Update paused for {} conflicts", conflicts.len()),
                );
                *state.conflicts.write().await = conflicts;
                state
                    .set_status("awaiting-conflict-resolution", "发现受管文件冲突", None)
                    .await;
                break;
            }
            Err(error) => {
                crate::logging::append(
                    &state.game_dir,
                    "ERROR",
                    format!("Update execution failed: {error}"),
                );
                state
                    .set_failure_status(&format!("更新失败：{error}"), "update")
                    .await;
                break;
            }
        }
    }
}

pub async fn initialize_updater(state: UpdaterState) {
    crate::logging::append(&state.game_dir, "INFO", "Updater initialization started");
    if let Err(error) = engine::recover_unfinished_transaction(&state.game_dir) {
        crate::logging::append(
            &state.game_dir,
            "ERROR",
            format!("Transaction recovery failed: {error}"),
        );
        state
            .set_failure_status(&format!("恢复未完成更新失败：{error}"), "update")
            .await;
        return;
    }
    match engine::inspect_client_version(&state.game_dir) {
        Ok(Some(version)) => {
            crate::logging::append(
                &state.game_dir,
                "INFO",
                format!("Detected current client version: {version}"),
            );
            *state.selected_version.write().await = Some(version.clone());
            match engine::check_next(&state, &version).await {
                Ok(check) if check.update_available => {
                    if state.mode == "bootstrap" {
                        crate::logging::append(
                            &state.game_dir,
                            "INFO",
                            format!(
                                "Bootstrap mode found update {} -> {}; starting automatically",
                                check.current_version, check.to_version
                            ),
                        );
                        state
                            .set_status_with_update(
                                "updating",
                                "客户端正在自动更新",
                                check.current_version,
                                check.to_version.clone(),
                                check.test_revision,
                            )
                            .await;
                        spawn_update(state.clone(), Some(check.to_version), None);
                    } else {
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
                }
                Ok(_) => {
                    state
                        .set_status("up-to-date", "客户端已是最新版本", None)
                        .await;
                    if state.mode == "bootstrap" {
                        exit_bootstrap_after_up_to_date_display(&state).await;
                    }
                }
                Err(error) => {
                    crate::logging::append(
                        &state.game_dir,
                        "ERROR",
                        format!("Update check failed: {error}"),
                    );
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
                crate::logging::append(
                    &state.game_dir,
                    "ERROR",
                    format!("Version listing failed: {error}"),
                );
                state.set_failure_status(&error.to_string(), "check").await;
            }
        },
        Err(error) => {
            crate::logging::append(
                &state.game_dir,
                "ERROR",
                format!("Client inspection failed: {error}"),
            );
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
    crate::logging::append(
        &state.game_dir,
        "START",
        format!("Full client install started: version={version}, mode={mode}"),
    );
    match engine::install_client_version(&state, &version, &mode).await {
        Ok(()) => {
            crate::logging::append(
                &state.game_dir,
                "SUCCESS",
                format!("Full client install finished: {version}"),
            );
            if *state.clean_downloads_after_install.read().await {
                let _ = engine::clean_downloads(&state.game_dir);
            }
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
            crate::logging::append(
                &state.game_dir,
                "ERROR",
                format!("Full client install failed: {error}"),
            );
            state
                .set_failure_status(&format!("客户端完整包覆盖失败：{error}"), "update")
                .await;
        }
    }
}

pub fn spawn_client_install(state: UpdaterState, version: String, mode: String) {
    tauri::async_runtime::spawn(execute_client_install(state, version, mode));
}
