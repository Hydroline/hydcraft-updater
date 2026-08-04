mod auth;
mod build_info;
mod commands;
mod contracts;
mod engine;
mod lifecycle;
mod logging;
mod state;
mod windows;

use auth::{desktop_login_url, exchange_desktop_code, logout_desktop, start_desktop_login};
use commands::{
    begin_update, cancel_conflict_resolution, clean_backups, clean_downloads,
    client_details_window_data, client_storage_info, client_version_options, desktop_identity,
    download_sources, hide_auth_window, hide_version_window, inspect_client,
    install_client_version, launch_client, open_client_details_window, open_external_url,
    open_version_window, pending_conflicts, play_failure_sound, recheck_update, resolve_conflicts,
    rollback_last_update, select_current_version, select_download_source, updater_context,
    updater_status,
};
use state::UpdaterState;
use std::{
    env,
    path::{Path, PathBuf},
};
use tauri::{Manager, RunEvent, WindowEvent};
use tauri_plugin_deep_link::DeepLinkExt;

pub fn run() {
    let arguments = env::args().collect::<Vec<_>>();
    if arguments
        .iter()
        .any(|argument| argument == "--identity-json")
    {
        println!("{}", build_info::identity_json());
        return;
    }
    let (game_dir, game_dir_source) = match argument_value(&arguments, "--game-dir") {
        Some(path) => (normalize_game_dir_path(PathBuf::from(path)), "command line"),
        None => match installed_updater_game_dir() {
            Some(path) => (path, "installed updater path"),
            None => (
                normalize_game_dir_path(env::current_dir().expect("current directory")),
                "current directory fallback",
            ),
        },
    };
    let console_origin = argument_value(&arguments, "--console-origin").unwrap_or_else(|| {
        if cfg!(debug_assertions) {
            "http://localhost:3001".into()
        } else {
            "https://console.hydcraft.cn".into()
        }
    });
    let mode = argument_value(&arguments, "--mode").unwrap_or_else(|| "manual".into());
    let log_game_dir = game_dir.clone();
    logging::append(
        &game_dir,
        "START",
        format!(
            "Updater started; version={}, commitSha={}, platform={}, mode={mode}, origin={console_origin}",
            build_info::current().version,
            build_info::current().commit_sha,
            build_info::current().platform,
        ),
    );
    logging::append(
        &game_dir,
        "INFO",
        format!(
            "Resolved game directory from {game_dir_source}: {}",
            game_dir.display()
        ),
    );
    let state = UpdaterState::new(game_dir, console_origin, mode);
    let update_state = state.clone();
    let callback_state = state.clone();
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.unminimize();
                let _ = main.show();
                let _ = main.set_focus();
            }
        }));
    }

    let app = builder
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let state = window.app_handle().state::<UpdaterState>();
                    let code = if state.mode == "bootstrap" { 1 } else { 0 };
                    logging::append(
                        &state.game_dir,
                        "RESULT",
                        format!("Updater window closed by user; exitCode={code}"),
                    );
                    window.app_handle().exit(code);
                }
            }
        })
        .manage(state)
        .setup(move |app| {
            #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
            app.deep_link().register_all()?;

            register_deep_link_listener(app, callback_state.clone());
            spawn_startup_tasks(app, update_state.clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            updater_status,
            updater_context,
            client_version_options,
            client_storage_info,
            clean_downloads,
            clean_backups,
            rollback_last_update,
            install_client_version,
            open_client_details_window,
            client_details_window_data,
            download_sources,
            inspect_client,
            open_version_window,
            hide_version_window,
            desktop_identity,
            pending_conflicts,
            resolve_conflicts,
            select_current_version,
            select_download_source,
            begin_update,
            cancel_conflict_resolution,
            recheck_update,
            play_failure_sound,
            launch_client,
            desktop_login_url,
            exchange_desktop_code,
            logout_desktop,
            start_desktop_login,
            hide_auth_window,
            open_external_url
        ])
        .build(tauri::generate_context!())
        .expect("HydCraft Updater failed to build");
    app.run(move |_app, event| match event {
        RunEvent::ExitRequested { code, .. } => {
            logging::append(
                &log_game_dir,
                "RESULT",
                format!("Updater exit requested; exitCode={}", code.unwrap_or(1)),
            );
        }
        RunEvent::Exit => logging::append(&log_game_dir, "END", "Updater process exited"),
        _ => {}
    });
}

fn register_deep_link_listener(app: &tauri::App, state: UpdaterState) {
    let app_handle = app.handle().clone();
    let listener_state = state.clone();
    app.deep_link().on_open_url(move |event| {
        let app_handle = app_handle.clone();
        let state = listener_state.clone();
        tauri::async_runtime::spawn(auth::process_deep_links(
            app_handle,
            state,
            event.urls().to_vec(),
        ));
    });

    if let Ok(Some(urls)) = app.deep_link().get_current() {
        tauri::async_runtime::spawn(auth::process_deep_links(app.handle().clone(), state, urls));
    }
}

fn spawn_startup_tasks(app: &tauri::App, state: UpdaterState) {
    let app_handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        state.bind_app(app_handle).await;
        let _ = auth::restore_desktop_session(state.clone()).await;
        lifecycle::initialize_updater(state).await;
    });
}

fn argument_value(arguments: &[String], name: &str) -> Option<String> {
    arguments
        .iter()
        .position(|argument| argument == name)
        .and_then(|index| arguments.get(index + 1))
        .cloned()
}

/// Derive the Minecraft directory only for an Updater launched from the installed
/// `.minecraft/.hydcraft/updater/<platform>/` layout. This keeps a manual
/// double-click attached to the existing client, while development builds and
/// copied binaries continue to use their working directory fallback.
fn installed_updater_game_dir() -> Option<PathBuf> {
    let executable = env::current_exe().ok()?;
    for platform_directory in executable.ancestors() {
        let platform = platform_directory.file_name()?.to_string_lossy();
        if !matches!(platform.as_ref(), "windows-x86_64" | "macos-universal") {
            continue;
        }

        let updater_directory = platform_directory.parent()?;
        if !path_name_is(updater_directory, "updater") {
            continue;
        }
        let hydcraft_directory = updater_directory.parent()?;
        if !path_name_is(hydcraft_directory, ".hydcraft") {
            continue;
        }
        let minecraft_directory = hydcraft_directory.parent()?;
        if !path_name_is(minecraft_directory, ".minecraft") || !minecraft_directory.is_dir() {
            continue;
        }
        return Some(minecraft_directory.to_path_buf());
    }
    None
}

fn path_name_is(path: &Path, expected: &str) -> bool {
    path.file_name()
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case(expected))
}

fn normalize_game_dir_path(path: PathBuf) -> PathBuf {
    let value = path.to_string_lossy();
    if !looks_like_windows_path(&value) {
        return path;
    }

    let mut normalized = String::with_capacity(value.len());
    let mut backslashes = 0;

    for character in value.chars() {
        if character == '\\' {
            backslashes += 1;
            continue;
        }

        if backslashes > 0 {
            let keep = if normalized.is_empty() {
                2.min(backslashes)
            } else {
                1
            };
            normalized.extend(std::iter::repeat_n('\\', keep));
            backslashes = 0;
        }
        normalized.push(character);
    }

    if backslashes > 0 {
        let keep = if normalized.is_empty() {
            2.min(backslashes)
        } else {
            1
        };
        normalized.extend(std::iter::repeat_n('\\', keep));
    }

    PathBuf::from(normalized)
}

fn looks_like_windows_path(value: &str) -> bool {
    let mut characters = value.chars();
    let drive_prefix = matches!(
        (characters.next(), characters.next(), characters.next()),
        (Some(letter), Some(':'), Some('\\')) if letter.is_ascii_alphabetic()
    );

    drive_prefix || value.starts_with("\\\\")
}
