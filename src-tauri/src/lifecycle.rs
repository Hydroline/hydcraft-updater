use crate::{engine, engine::EngineError, state::UpdaterState};
use std::time::Duration;

const BOOTSTRAP_AUTOMATIC_ACTION_DURATION_SECONDS: u8 = 10;
const BOOTSTRAP_AUTOMATIC_ACTION_DURATION: Duration =
    Duration::from_secs(BOOTSTRAP_AUTOMATIC_ACTION_DURATION_SECONDS as u64);
const BOOTSTRAP_UP_TO_DATE_LAUNCH_DURATION_SECONDS: u8 = 3;
const BOOTSTRAP_UP_TO_DATE_LAUNCH_DURATION: Duration =
    Duration::from_secs(BOOTSTRAP_UP_TO_DATE_LAUNCH_DURATION_SECONDS as u64);

fn is_bootstrap_launch_phase(phase: &str) -> bool {
    matches!(phase, "ready" | "up-to-date")
}

async fn schedule_bootstrap_launch(
    state: UpdaterState,
    deadline: std::time::Instant,
    duration_seconds: u8,
) {
    let initial = state.status.read().await.clone();
    if !is_bootstrap_launch_phase(&initial.phase)
        || !state
            .bootstrap_auto_countdown_is_active(deadline, &initial.phase)
            .await
    {
        return;
    }
    let phase = initial.phase;
    let message = initial.message;

    for remaining_seconds in (1..duration_seconds).rev() {
        tokio::time::sleep(Duration::from_secs(1)).await;
        if !state
            .bootstrap_auto_countdown_is_active(deadline, &phase)
            .await
        {
            return;
        }
        state
            .set_status(&phase, &message, Some(remaining_seconds))
            .await;
    }

    tokio::time::sleep(Duration::from_secs(1)).await;
    if !state
        .consume_bootstrap_auto_countdown(deadline, &phase)
        .await
    {
        return;
    }
    crate::logging::append(
        &state.game_dir,
        "SUCCESS",
        "Bootstrap launch countdown completed; exiting updater",
    );
    state.exit_process(0).await;
}

pub async fn begin_bootstrap_launch_countdown(state: UpdaterState) {
    let status = state.status.read().await.clone();
    if !is_bootstrap_launch_phase(&status.phase) {
        return;
    }
    let (duration_seconds, duration) = if status.phase == "up-to-date" {
        (
            BOOTSTRAP_UP_TO_DATE_LAUNCH_DURATION_SECONDS,
            BOOTSTRAP_UP_TO_DATE_LAUNCH_DURATION,
        )
    } else {
        (
            BOOTSTRAP_AUTOMATIC_ACTION_DURATION_SECONDS,
            BOOTSTRAP_AUTOMATIC_ACTION_DURATION,
        )
    };
    state
        .set_status(&status.phase, &status.message, Some(duration_seconds))
        .await;
    let deadline = state.arm_bootstrap_auto_countdown(duration).await;
    tauri::async_runtime::spawn(schedule_bootstrap_launch(state, deadline, duration_seconds));
}

async fn schedule_bootstrap_update(
    state: UpdaterState,
    check: engine::ClientUpdateCheck,
    deadline: std::time::Instant,
) {
    for remaining_seconds in (1..BOOTSTRAP_AUTOMATIC_ACTION_DURATION_SECONDS).rev() {
        tokio::time::sleep(Duration::from_secs(1)).await;
        if !state
            .bootstrap_auto_countdown_is_active(deadline, "awaiting-update-decision")
            .await
        {
            return;
        }
        state
            .set_status_with_update_countdown(
                "发现客户端更新",
                check.current_version.clone(),
                check.to_version.clone(),
                check.test_revision,
                remaining_seconds,
            )
            .await;
    }

    tokio::time::sleep(Duration::from_secs(1)).await;
    if !state
        .consume_bootstrap_auto_countdown(deadline, "awaiting-update-decision")
        .await
    {
        return;
    }
    state.set_status("updating", "正在更新客户端", None).await;
    spawn_update(state, Some(check.to_version), None);
}

pub async fn present_update_check(state: UpdaterState, check: engine::ClientUpdateCheck) {
    if check.update_available {
        if state.mode == "bootstrap" {
            crate::logging::append(
                &state.game_dir,
                "INFO",
                format!(
                    "Bootstrap mode found update {} -> {}; starting automatic countdown",
                    check.current_version, check.to_version
                ),
            );
            state
                .set_status_with_update_countdown(
                    "发现客户端更新",
                    check.current_version.clone(),
                    check.to_version.clone(),
                    check.test_revision,
                    BOOTSTRAP_AUTOMATIC_ACTION_DURATION_SECONDS,
                )
                .await;
            let deadline = state
                .arm_bootstrap_auto_countdown(BOOTSTRAP_AUTOMATIC_ACTION_DURATION)
                .await;
            tauri::async_runtime::spawn(schedule_bootstrap_update(state, check, deadline));
        } else {
            state
                .set_status_with_update(
                    "awaiting-update-decision",
                    "发现客户端更新",
                    check.current_version,
                    check.to_version,
                    check.test_revision,
                )
                .await;
        }
    } else {
        state
            .set_status("up-to-date", "客户端已是最新版本", None)
            .await;
        if state.mode == "bootstrap" {
            begin_bootstrap_launch_countdown(state).await;
        }
    }
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
                    begin_bootstrap_launch_countdown(state.clone()).await;
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
                Ok(check) => present_update_check(state.clone(), check).await,
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
            if state.mode == "bootstrap" {
                begin_bootstrap_launch_countdown(state.clone()).await;
            }
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
