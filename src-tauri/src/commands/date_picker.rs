//! 日期选择弹窗：独立置顶小窗口承载日历（原生 flyout 的等价实现——HTML 内容
//! 画不出窗口边界，只有独立窗口才能真正浮在主窗口外）。
//!
//! 弹窗**常驻复用**：首次点击创建，之后失焦只 hide、再次打开 show——webview
//! 不重新加载（每次重建要完整引导一遍前端，dev 下数百模块，点击后要等数秒）。

use std::sync::Mutex;

use tauri::{
    webview::PageLoadEvent, AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder,
};

use crate::domain::error::CommandError;

pub const DATE_PICKER_LABEL: &str = "date-picker";
pub const DATE_PAYLOAD_EVENT: &str = "date-payload";

/// 选中值中转：主窗口写入，弹窗挂载时读取（paim 全屏查看窗口同款模式）。
#[derive(Default)]
pub struct DatePickerPayload(Mutex<Option<String>>);

fn win_error(e: tauri::Error) -> CommandError {
    CommandError::Message(format!("窗口操作失败：{e}"))
}

/// 打开日期选择弹窗（主窗口在输入框下方定位后调用；x/y 为逻辑像素屏幕坐标）。
/// 窗口常驻：已存在则改位置、推新值、show + focus，毫秒级。
/// 必须是 async 命令（WebviewWindowBuilder::build 在同步命令里于 Windows 上死锁，
/// 官方文档警告）。
pub async fn open_date_picker_inner(
    app: &AppHandle,
    x: f64,
    y: f64,
    date: Option<String>,
) -> Result<(), CommandError> {
    let payload = app.state::<DatePickerPayload>();
    *payload
        .0
        .lock()
        .map_err(|e| CommandError::Store(e.to_string()))? = date.clone();

    if let Some(w) = app.get_webview_window(DATE_PICKER_LABEL) {
        let _ = w.set_position(tauri::LogicalPosition::new(x, y));
        let _ = app.emit_to(DATE_PICKER_LABEL, DATE_PAYLOAD_EVENT, date);
        let _ = w.show();
        let _ = w.set_focus();
        return Ok(());
    }

    let win =
        WebviewWindowBuilder::new(app, DATE_PICKER_LABEL, WebviewUrl::App("index.html".into()))
            .title("选择日期")
            .inner_size(240.0, 236.0)
            .decorations(false)
            .resizable(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(true)
            .visible(false) // 页面加载完成后再显示，避免白屏/半成品闪烁
            .position(x, y)
            .on_page_load(|w, event| {
                if event.event() == PageLoadEvent::Finished {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            })
            .build()
            .map_err(win_error)?;

    let hider = win.clone();
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::Focused(false) = event {
            let _ = hider.hide();
        }
    });
    Ok(())
}

/// 打开日期选择弹窗（前端点击日期框调用）。必须是 async 命令
/// （WebviewWindowBuilder::build 在同步命令里于 Windows 上死锁，官方文档警告）。
#[tauri::command]
#[specta::specta]
pub async fn open_date_picker(
    app: AppHandle,
    x: f64,
    y: f64,
    date: Option<String>,
) -> Result<(), CommandError> {
    open_date_picker_inner(&app, x, y, date).await
}

/// 弹窗挂载时读取初始选中值（ISO 字符串，无则返回 None）。
#[tauri::command]
#[specta::specta]
pub fn get_date_picker_payload(payload: State<'_, DatePickerPayload>) -> Option<String> {
    payload.0.lock().unwrap().clone()
}

/// 选中日期后由弹窗调用：隐藏自身（窗口保留供复用）。
#[tauri::command]
#[specta::specta]
pub async fn close_date_picker(app: AppHandle) -> Result<(), CommandError> {
    if let Some(w) = app.get_webview_window(DATE_PICKER_LABEL) {
        let _ = w.hide();
    }
    Ok(())
}
