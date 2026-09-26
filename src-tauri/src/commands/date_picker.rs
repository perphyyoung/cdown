//! 日期选择弹窗：独立置顶小窗口承载日历（原生 flyout 的等价实现——HTML 内容
//! 画不出窗口边界，只有独立窗口才能真正浮在主窗口外）。

use std::sync::Mutex;

use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::domain::error::CommandError;

pub const DATE_PICKER_LABEL: &str = "date-picker";

/// 选中值中转：主窗口写入，弹窗挂载时读取（paim 全屏查看窗口同款模式）。
#[derive(Default)]
pub struct DatePickerPayload(Mutex<Option<String>>);

fn win_error(e: tauri::Error) -> CommandError {
    CommandError::Message(format!("窗口操作失败：{e}"))
}

/// 打开日期选择弹窗（主窗口在输入框下方定位后调用；x/y 为逻辑像素屏幕坐标）。
/// 已存在则改位置并复用。必须是 async 命令（WebviewWindowBuilder::build 在
/// 同步命令里于 Windows 上死锁，官方文档警告）。
#[tauri::command]
#[specta::specta]
pub async fn open_date_picker(
    app: AppHandle,
    payload: State<'_, DatePickerPayload>,
    x: f64,
    y: f64,
    date: Option<String>,
) -> Result<(), CommandError> {
    *payload
        .0
        .lock()
        .map_err(|e| CommandError::Store(e.to_string()))? = date;

    if let Some(w) = app.get_webview_window(DATE_PICKER_LABEL) {
        let _ = w.set_position(tauri::LogicalPosition::new(x, y));
        let _ = w.set_focus();
        return Ok(());
    }

    let win = WebviewWindowBuilder::new(
        &app,
        DATE_PICKER_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("选择日期")
    .inner_size(240.0, 236.0)
    .decorations(false)
    .resizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(true)
    .position(x, y)
    .build()
    .map_err(win_error)?;

    // 失焦即关（原生 flyout 行为）：点主窗口/其它窗口任意处关闭
    let closer = win.clone();
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::Focused(false) = event {
            let _ = closer.close();
        }
    });
    Ok(())
}

/// 弹窗挂载时读取初始选中值（ISO 字符串，无则返回 None）。
#[tauri::command]
#[specta::specta]
pub fn get_date_picker_payload(payload: State<'_, DatePickerPayload>) -> Option<String> {
    payload.0.lock().unwrap().clone()
}

/// 选中日期后由弹窗调用：关闭自身。
#[tauri::command]
#[specta::specta]
pub async fn close_date_picker(app: AppHandle) -> Result<(), CommandError> {
    if let Some(w) = app.get_webview_window(DATE_PICKER_LABEL) {
        let _ = w.close();
    }
    Ok(())
}
