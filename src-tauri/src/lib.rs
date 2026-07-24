mod auth;
mod commands;
mod contracts;
mod engine;
mod lifecycle;
mod state;
mod windows;

use auth::{desktop_login_url, exchange_desktop_code, logout_desktop, start_desktop_login};
use commands::{
    begin_update, client_version_options, desktop_identity, download_sources, hide_auth_window,
    hide_version_window, inspect_client, launch_client, open_external_url, open_version_window,
    pending_conflicts, recheck_update, resolve_conflicts, select_current_version,
    select_download_source, skip_update, updater_context, updater_status,
};
use state::UpdaterState;
use std::{env, path::PathBuf};
use tauri::Manager;
use tauri_plugin_deep_link::DeepLinkExt;

pub fn run() {
    let arguments = env::args().collect::<Vec<_>>();
    let game_dir = argument_value(&arguments, "--game-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().expect("current directory"));
    let console_origin = argument_value(&arguments, "--console-origin")
        .unwrap_or_else(|| "https://console.hydcraft.cn".into());
    let mode = argument_value(&arguments, "--mode").unwrap_or_else(|| "manual".into());
    let state = UpdaterState::new(game_dir, console_origin, mode);
    let update_state = state.clone();
    let callback_state = state.clone();
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.show();
                let _ = main.set_focus();
            }
        }));
    }

    builder
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
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
            recheck_update,
            skip_update,
            launch_client,
            desktop_login_url,
            exchange_desktop_code,
            logout_desktop,
            start_desktop_login,
            hide_auth_window,
            open_external_url
        ])
        .run(tauri::generate_context!())
        .expect("HydCraft Updater failed to run");
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
