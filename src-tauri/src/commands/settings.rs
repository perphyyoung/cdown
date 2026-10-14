//! 设置命令：紧急度分级等全局设置；设置窗口的创建/唤起。
//! 表格列宽不属于设置，见 commands/columns.rs。

use std::collections::BTreeMap;

use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, State, WebviewUrl,
    WebviewWindowBuilder,
};

use crate::commands::hotkey;
use crate::domain::error::CommandError;
use crate::domain::model::Settings;
use crate::infra::font_map;
use crate::infra::store::{data_dir, Store, StoreData};
use crate::log_warn;

pub const SETTINGS_WINDOW_LABEL: &str = "settings";
/// 复用打开通知：设置窗口收到后重新量高再 reveal（首次打开由前端挂载流程自己走同一条路）
pub const SETTINGS_OPEN_EVENT: &str = "settings-open";
/// 设置窗口宽度（设计基准）：宽度可由用户拖拽，高度永远由内容决定
const SETTINGS_WIDTH: f64 = 640.0;
/// 创建时的兜底高度：只在 reveal 校准之前用得到（估算全部设置行的内容高，宁可略大——
/// 内容顶对齐，略大会表现为向下"落定"，略小则是从空面板里"长出来"）
const SETTINGS_FALLBACK_HEIGHT: f64 = 760.0;
/// 锚点与设置窗口上边缘的间距
const ANCHOR_GAP: f64 = 4.0;
/// 设置窗口下边缘与工作区底边的最小间距
const BOTTOM_MARGIN: f64 = 16.0;
/// 锚定后至少留在工作区内的可见高度
const MIN_VISIBLE_HEIGHT: f64 = 160.0;

/// 打开设置窗口。窗口**常驻复用**：首次创建（visible(false)，等前端量好高度再 reveal），
/// 之后只 show + 通知前端重新量高——webview 不重新加载，重开毫秒级（与日期选择弹窗
/// commands/date_picker.rs 同构）。真正的销毁只发生在进程退出。
///
/// anchor：(逻辑像素) 屏幕坐标锚点，通常取主窗口 ⚙ 的右下角——设置窗口右边缘对齐锚点、
/// 向下展开；None（托盘入口）回落主窗口右上角，再取不到显示器信息则居中。
///
/// 必须由 async 命令/独立线程调用：WebviewWindowBuilder::build() 在 Windows 上于同步
/// 命令、事件处理器所在线程里会死锁（官方文档明确警告）。
pub fn open_settings_window(
    app: &AppHandle,
    anchor: Option<(f64, f64)>,
) -> Result<(), CommandError> {
    match app.get_webview_window(SETTINGS_WINDOW_LABEL) {
        Some(w) => {
            // 已可见 = 再点一次 ⚙：只前置聚焦，不打断正在编辑的内容，也不必重新量高
            let visible = w.is_visible().unwrap_or(false);
            let _ = w.show();
            let _ = w.unminimize();
            let _ = w.set_focus();
            if !visible {
                app.emit_to(SETTINGS_WINDOW_LABEL, SETTINGS_OPEN_EVENT, ())
                    .map_err(|e| CommandError::Message(format!("通知设置窗口失败：{e}")))?;
            }
            Ok(())
        }
        None => {
            let mut builder = WebviewWindowBuilder::new(
                app,
                SETTINGS_WINDOW_LABEL,
                WebviewUrl::App("index.html".into()),
            )
            .title("cdown 设置")
            .inner_size(SETTINGS_WIDTH, SETTINGS_FALLBACK_HEIGHT)
            .decorations(false)
            // 关掉原生缩放：高度恒等于内容，任何方向的原生拖拽都会留下空白区域。
            // 无边框窗口的缩放热区由 tauri-runtime-wry 的 TAURI_DRAG_RESIZE_BORDERS 子窗口
            // 提供，而它对 WM_NCHITTEST 的第一件事就是 `style & WS_SIZEBOX`——
            // resizable(false) 后该子窗口直接不创建，底边/四角彻底不是热区（也没有残留的
            // 缩放光标）。宽度由前端自绘的左右把手驱动（见 SettingsPage.vue）。
            .resizable(false)
            // 双击自绘标题栏会触发注入脚本的 internal_toggle_maximize（Tauri 侧判 is_maximizable），
            // 不关掉就会被撑到屏幕大小、露出大片空白
            .maximizable(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible(false); // 前端量好高度再显示：避免空窗闪现与出现后再改尺寸的跳变
            builder = match anchored_position(app, anchor) {
                Some(pos) => builder.position(pos.x, pos.y),
                None => builder.center(),
            };
            let win = builder
                .build()
                .map_err(|e| CommandError::Message(format!("打开设置窗口失败：{e}")))?;
            // 关窗即隐藏（Alt+F4/系统关闭同理）：窗口常驻，重开不必重建 webview
            let hider = win.clone();
            win.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = hider.hide();
                }
            });
            Ok(())
        }
    }
}

/// 锚点 → 设置窗口左上角（逻辑像素）：右边缘对齐锚点、向下展开，并钳制在所在显示器的
/// 工作区内（不压任务栏）。取不到锚点或显示器信息时返回 None（调用方回落居中）。
fn anchored_position(app: &AppHandle, anchor: Option<(f64, f64)>) -> Option<LogicalPosition<f64>> {
    let main = app.get_webview_window("main")?;
    let scale = main.scale_factor().ok()?;
    let (ax, ay) = match anchor {
        Some(point) => point,
        // 托盘入口没有点击位置，用主窗口右上角当锚点
        None => {
            let pos = main.outer_position().ok()?.to_logical::<f64>(scale);
            let size = main.outer_size().ok()?.to_logical::<f64>(scale);
            (pos.x + size.width, pos.y)
        }
    };
    // ⚙ 必然在主窗口所在显示器上，取主窗口的显示器即可（跨屏缩放按该屏换算）
    let monitor = main.current_monitor().ok().flatten()?;
    let work_area = monitor.work_area();
    let area_pos = work_area.position.to_logical::<f64>(scale);
    let area_size = work_area.size.to_logical::<f64>(scale);
    let left_min = area_pos.x;
    let left_max = (area_pos.x + area_size.width - SETTINGS_WIDTH).max(left_min);
    let top_min = area_pos.y;
    let top_max = (area_pos.y + area_size.height - MIN_VISIBLE_HEIGHT).max(top_min);
    Some(LogicalPosition::new(
        (ax - SETTINGS_WIDTH).clamp(left_min, left_max),
        (ay + ANCHOR_GAP).clamp(top_min, top_max),
    ))
}

/// 还能给设置窗口多少高度（逻辑像素）：工作区底边 - 窗口上边缘 - 底部余量。
/// 内容比它还高时窗口就夹在这里，多出来的内容由前端的内容区纵向滚动兜住
/// （见 SettingsPage.vue）——屏幕高度有限，窄窗口的内容可以轻松超过一屏。
/// 取不到显示器/位置信息时返回 None（不限制）。
fn max_content_height(win: &tauri::WebviewWindow) -> Option<f64> {
    let scale = win.scale_factor().ok()?;
    let top = win.outer_position().ok()?.to_logical::<f64>(scale).y;
    let monitor = win.current_monitor().ok().flatten()?;
    let work_area = monitor.work_area();
    let area_pos = work_area.position.to_logical::<f64>(scale);
    let area_size = work_area.size.to_logical::<f64>(scale);
    let bottom = area_pos.y + area_size.height;
    Some((bottom - top - BOTTOM_MARGIN).max(MIN_VISIBLE_HEIGHT))
}

/// 显示设置窗口（前端量好内容高度后调用），返回本次的可用高度上限（None = 不限制），
/// 供前端后续改宽时继续夹取。
/// 必须先 show 再 set_size：隐藏态 set_size 在 Windows 上会多算一个 caption 高（与主窗口
/// 重启长高同源，见 lib.rs 的二次恢复注释），只有显示后的尺寸才准。宽度沿用当前值
/// （用户可拖），高度 = min(内容高, 可用高)——两个权威各管一轴，不再互相覆盖。
#[tauri::command]
#[specta::specta]
pub fn reveal_settings_window(
    app: AppHandle,
    height: Option<f64>,
) -> Result<Option<f64>, CommandError> {
    let win = app
        .get_webview_window(SETTINGS_WINDOW_LABEL)
        .ok_or_else(|| CommandError::Message("设置窗口不存在".into()))?;
    win.show()
        .map_err(|e| CommandError::Message(format!("显示设置窗口失败：{e}")))?;
    let max_height = max_content_height(&win);
    let scale = win.scale_factor().unwrap_or(1.0);
    let inner = win
        .inner_size()
        .map_err(|e| CommandError::Message(format!("读取设置窗口尺寸失败：{e}")))?
        .to_logical::<f64>(scale);
    if let Some(natural) = height.filter(|h| *h >= 1.0) {
        let target = match max_height {
            Some(max) => natural.min(max),
            None => natural,
        };
        if (inner.height - target).abs() > 1.0 {
            win.set_size(LogicalSize::new(inner.width, target))
                .map_err(|e| CommandError::Message(format!("校准设置窗口高度失败：{e}")))?;
        }
    }
    let _ = win.set_focus();
    Ok(max_height)
}

/// 隐藏设置窗口（✕ 与 Esc 调用）：窗口常驻复用，不销毁。
#[tauri::command]
#[specta::specta]
pub fn hide_settings_window(app: AppHandle) -> Result<(), CommandError> {
    if let Some(win) = app.get_webview_window(SETTINGS_WINDOW_LABEL) {
        win.hide()
            .map_err(|e| CommandError::Message(format!("隐藏设置窗口失败：{e}")))?;
    }
    Ok(())
}

/// 打开设置窗口（前端 ⚙ 按钮调用；anchor 为 ⚙ 的右下角屏幕坐标，托盘入口不传）。
/// 必须是 async 命令：WebviewWindowBuilder::build() 在 Windows 上于同步命令中
/// 会死锁（官方文档明确警告，命令/事件处理器里要改用 async 或独立线程）。
#[tauri::command]
#[specta::specta]
pub async fn open_settings(
    app: AppHandle,
    anchor_x: Option<f64>,
    anchor_y: Option<f64>,
) -> Result<(), CommandError> {
    open_settings_window(&app, anchor_x.zip(anchor_y))
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
    persist_settings(&app, &store, settings)
}

/// 还原所有配置为默认值（分级/置顶/热键/背景/字体），倒计时数据与表格列宽不动。
/// 必须与 set_settings 走同一条持久化路径，默认热键才会立即重新注册生效。
#[tauri::command]
#[specta::specta]
pub fn reset_settings(app: AppHandle, store: State<'_, Store>) -> Result<Settings, CommandError> {
    persist_settings(&app, &store, Settings::default())
}

/// 归一化 → 热键校验/注册 → 落盘；落盘失败把热键注册回滚到旧值，
/// 避免内存热键与 cdown.json 不一致。
fn persist_settings(
    app: &AppHandle,
    store: &State<'_, Store>,
    mut settings: Settings,
) -> Result<Settings, CommandError> {
    settings = settings.normalized();
    // 热键先校验规范化（非法即整次保存失败），再注册、最后落盘：
    // 顺序不能反——先落盘会把一个没生效的键写进 cdown.json；先注册则失败时旧键仍可用。
    settings.hotkey = hotkey::canonicalize(settings.hotkey.as_deref())?;
    let prev = store.read()?.settings.hotkey;
    hotkey::apply(app, settings.hotkey.as_deref())?;
    let saved = store.mutate(|d: &mut StoreData| -> Result<(), CommandError> {
        d.settings = settings.clone();
        Ok(())
    });
    if let Err(e) = saved {
        if let Err(back) = hotkey::apply(app, prev.as_deref()) {
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
