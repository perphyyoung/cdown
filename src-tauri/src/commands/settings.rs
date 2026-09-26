//! 设置命令：紧急度分级与表格列宽；设置窗口的创建/唤起。

use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::domain::error::CommandError;
use crate::domain::model::Settings;
use crate::infra::store::{Store, StoreData};

pub const SETTINGS_WINDOW_LABEL: &str = "settings";

/// 打开设置窗口：已存在则唤起，不存在则创建（置顶 + 原生标题栏，
/// 方便调整设置时对照主面板样式）。tray 与前端 ⚙ 共用此入口。
pub fn open_settings_window(app: &AppHandle) -> Result<(), CommandError> {
    match app.get_webview_window(SETTINGS_WINDOW_LABEL) {
        Some(w) => {
            let _ = w.show();
            let _ = w.unminimize();
            let _ = w.set_focus();
            Ok(())
        }
        None => WebviewWindowBuilder::new(
            app,
            SETTINGS_WINDOW_LABEL,
            WebviewUrl::App("index.html".into()),
        )
        .title("cdown 设置")
        .inner_size(640.0, 280.0)
        .resizable(true)
        .always_on_top(true)
        .center()
        .build()
        .map(|_| ())
        .map_err(|e| CommandError::Message(format!("打开设置窗口失败：{e}"))),
    }
}

/// 打开设置窗口（前端 ⚙ 按钮调用）。
/// 必须是 async 命令：WebviewWindowBuilder::build() 在 Windows 上于同步命令中
/// 会死锁（官方文档明确警告，命令/事件处理器里要改用 async 或独立线程）。
#[tauri::command]
#[specta::specta]
pub async fn open_settings(app: AppHandle) -> Result<(), CommandError> {
    open_settings_window(&app)
}

#[tauri::command]
#[specta::specta]
pub fn get_settings(store: State<'_, Store>) -> Result<Settings, CommandError> {
    Ok(store.read()?.settings)
}

#[tauri::command]
#[specta::specta]
pub fn set_settings(store: State<'_, Store>, settings: Settings) -> Result<Settings, CommandError> {
    let mut settings = settings.normalized();
    settings.column_widths = settings.column_widths.sanitized();
    store.mutate(|d: &mut StoreData| -> Result<(), CommandError> {
        d.settings = settings.clone();
        Ok(())
    })?;
    Ok(settings)
}
