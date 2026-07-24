use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub fn open_version_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("version")
        .or_else(|| {
            WebviewWindowBuilder::new(&app, "version", WebviewUrl::App("index.html".into()))
                .title("HydCraft 选择客户端版本")
                .inner_size(420.0, 500.0)
                .center()
                .resizable(false)
                .decorations(false)
                .visible(false)
                .build()
                .ok()
        })
        .ok_or_else(|| "无法创建版本选择窗口".to_string())?;
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
}

pub fn hide_version_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("version") {
        window.hide().map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn get_or_create_auth_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    if let Some(window) = app.get_webview_window("auth") {
        return Ok(window);
    }
    WebviewWindowBuilder::new(app, "auth", WebviewUrl::App("index.html".into()))
        .title("HydCraft 账户验证")
        .inner_size(420.0, 500.0)
        .center()
        .resizable(false)
        .decorations(false)
        .visible(false)
        .build()
        .map_err(|error| error.to_string())
}

pub fn show_auth_window(app: &AppHandle) -> Result<(), String> {
    let window = get_or_create_auth_window(app)?;
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
}

pub fn hide_auth_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("auth") {
        window.hide().map_err(|error| error.to_string())?;
    }
    Ok(())
}
