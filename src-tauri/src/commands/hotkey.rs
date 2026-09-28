//! 全局热键：解析/注册/改键，以及热键触发时唤起主窗口。
//!
//! 插件只在 Rust 侧调用（不走前端 IPC），故 capabilities 无需开 global-shortcut 权限；
//! 键位不在插件构建期注册（构建期注册失败会让启动直接失败），而是按设置在这里注册。

use std::sync::Mutex;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut};

use crate::domain::error::CommandError;
use crate::{log_info, log_warn};

/// 当前已注册的热键（规范串 + 解析结果）；None = 未注册（用户关闭热键）
#[derive(Default)]
pub struct RegisteredHotkey(Mutex<Option<(String, Shortcut)>>);

/// 解析并规范化 accelerator：修饰键固定顺序 Ctrl/Alt/Shift/Super，
/// 主键去掉 Web 键名里的 Key/Digit 前缀（KeyC → C、Digit1 → 1），其余保留（F8、Space…）。
/// 空串表示关闭热键（Ok(None)）；无修饰键或无法识别的串报错。
fn parse_accelerator(raw: &str) -> Result<Option<(String, Shortcut)>, CommandError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    // 用插件同一个解析器校验，不自己写正则（串格式以 global-hotkey 为准）
    let shortcut: Shortcut = raw
        .parse()
        .map_err(|e| CommandError::Invalid(format!("无法识别快捷键「{raw}」：{e}")))?;
    if shortcut.mods.is_empty() {
        return Err(CommandError::Invalid(
            "全局热键需包含 Ctrl / Alt / Shift 中至少一个修饰键".into(),
        ));
    }
    Ok(Some((canonical(&shortcut), shortcut)))
}

/// 校验并规范化：供命令层在落盘前把用户输入统一成规范串（None = 关闭热键）
pub fn canonicalize(hotkey: Option<&str>) -> Result<Option<String>, CommandError> {
    match hotkey {
        Some(raw) => Ok(parse_accelerator(raw)?.map(|(name, _)| name)),
        None => Ok(None),
    }
}

fn canonical(shortcut: &Shortcut) -> String {
    let mut parts: Vec<&str> = Vec::with_capacity(2);
    for (flag, label) in [
        (Modifiers::CONTROL, "Ctrl"),
        (Modifiers::ALT, "Alt"),
        (Modifiers::SHIFT, "Shift"),
        (Modifiers::SUPER, "Super"),
    ] {
        if shortcut.mods.contains(flag) {
            parts.push(label);
        }
    }
    let key = shortcut.key.to_string();
    let main_key = key
        .strip_prefix("Key")
        .or_else(|| key.strip_prefix("Digit"))
        .unwrap_or(key.as_str());
    parts.push(main_key);
    parts.join("+")
}

/// 应用热键设置：先注册新键、成功后再注销旧键（新键失败时旧键保持可用）。
/// 与当前键相同时直接返回——列宽等其它设置每次保存都会走到这里。
pub fn apply(app: &AppHandle, hotkey: Option<&str>) -> Result<(), CommandError> {
    let next = match hotkey {
        Some(raw) => parse_accelerator(raw)?,
        None => None,
    };
    let state = app.state::<RegisteredHotkey>();
    let mut current = state.0.lock().unwrap_or_else(|e| e.into_inner());
    let unchanged = match (current.as_ref(), next.as_ref()) {
        (None, None) => true,
        (Some((cur, _)), Some((want, _))) => cur == want,
        _ => false,
    };
    if unchanged {
        return Ok(());
    }

    let shortcuts = app.global_shortcut();
    if let Some((want, shortcut)) = &next {
        shortcuts.register(*shortcut).map_err(|e| {
            CommandError::Message(format!("热键注册失败，可能已被其它程序占用：{e}"))
        })?;
        log_info!("全局热键已注册：{want}");
    }
    if let Some((cur, shortcut)) = current.take() {
        if let Err(e) = shortcuts.unregister(shortcut) {
            log_warn!("注销旧全局热键 {cur} 失败：{e}");
        }
    }
    *current = next;
    Ok(())
}

/// 把主窗口从托盘隐藏/最小化状态唤起（与托盘「显示」同一动作）
pub fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

#[cfg(test)]
#[path = "hotkey.test.rs"]
mod tests;
