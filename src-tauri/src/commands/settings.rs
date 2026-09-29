//! 设置命令：紧急度分级与表格列宽；设置窗口的创建/唤起。

use std::collections::BTreeMap;

use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::commands::hotkey;
use crate::domain::error::CommandError;
use crate::domain::model::Settings;
use crate::infra::font_map;
use crate::infra::store::{data_dir, Store, StoreData};
use crate::log_warn;

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
        .decorations(false)
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
pub fn set_settings(
    app: AppHandle,
    store: State<'_, Store>,
    settings: Settings,
) -> Result<Settings, CommandError> {
    let mut settings = settings.normalized();
    settings.column_widths = settings.column_widths.sanitized();
    // 热键先校验规范化（非法即整次保存失败），再注册、最后落盘：
    // 顺序不能反——先落盘会把一个没生效的键写进 cdown.json；先注册则失败时旧键仍可用。
    settings.hotkey = hotkey::canonicalize(settings.hotkey.as_deref())?;
    let prev = store.read()?.settings.hotkey;
    hotkey::apply(&app, settings.hotkey.as_deref())?;
    let saved = store.mutate(|d: &mut StoreData| -> Result<(), CommandError> {
        d.settings = settings.clone();
        Ok(())
    });
    if let Err(e) = saved {
        // 落盘失败：把热键注册回滚到旧值，避免内存热键与 cdown.json 不一致
        if let Err(back) = hotkey::apply(&app, prev.as_deref()) {
            log_warn!("全局热键回滚失败：{back}");
        }
        return Err(e);
    }
    Ok(settings)
}

/// 字体中文名映射（数据目录 `font-family-map.toml`，缺失时写入默认模板）。
/// 仅用于设置页下拉的显示文案，失败回落内置默认映射、不报错。
#[tauri::command]
#[specta::specta]
pub fn get_font_family_map(app: AppHandle) -> BTreeMap<String, String> {
    font_map::load_or_create(&data_dir(&app))
}
