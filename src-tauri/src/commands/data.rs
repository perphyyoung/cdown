//! 倒计时与设置的独立导出/导入：各成一个 JSON 文件，互不合并。

use std::fs;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::commands::hotkey;
use crate::domain::error::CommandError;
use crate::domain::model::{CountdownItem, Settings};
use crate::infra::store::{Store, StoreData};
use crate::log_warn;

const EXPORT_FORMAT: &str = "cdown-export";
const EXPORT_VERSION: u32 = 1;
const KIND_ITEMS: &str = "items";
const KIND_SETTINGS: &str = "settings";

/// 导出文件内的时间戳，精确到秒
fn now_stamp() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

#[derive(Debug, Serialize, specta::Type)]
struct ItemsExport<'a> {
    app: &'static str,
    version: u32,
    kind: &'static str,
    exported_at: String,
    items: &'a [CountdownItem],
}

#[derive(Debug, Serialize, specta::Type)]
struct SettingsExport<'a> {
    app: &'static str,
    version: u32,
    kind: &'static str,
    exported_at: String,
    settings: &'a Settings,
}

/// 导入文件统一外壳：按 kind 区分内容，kind 不匹配即报错。
#[derive(Debug, Deserialize)]
struct ImportEnvelope {
    #[serde(default)]
    kind: String,
    #[serde(default)]
    items: Option<Vec<CountdownItem>>,
    #[serde(default)]
    settings: Option<Settings>,
}

#[derive(Debug, Serialize, specta::Type)]
pub struct ImportItemsResult {
    /// 导入的倒计时条数
    pub items: usize,
}

fn read_envelope(path: &str) -> Result<ImportEnvelope, CommandError> {
    let raw = fs::read_to_string(path)
        .map_err(|e| CommandError::Message(format!("读取导入文件失败：{e}")))?;
    serde_json::from_str(&raw)
        .map_err(|e| CommandError::Invalid(format!("导入文件不是有效的 cdown 备份：{e}")))
}

/// 逐条严格校验，任何一条无效即整体失败，不产生半导入状态。
fn validate_items(items: &[CountdownItem]) -> Result<(), CommandError> {
    for (i, item) in items.iter().enumerate() {
        if item.title.trim().is_empty() {
            return Err(CommandError::Invalid(format!(
                "第 {} 条记录名称为空",
                i + 1
            )));
        }
        if chrono::NaiveDate::parse_from_str(&item.target_date, "%Y-%m-%d").is_err() {
            return Err(CommandError::Invalid(format!(
                "第 {} 条记录目标日期无效：{}",
                i + 1,
                item.target_date
            )));
        }
    }
    Ok(())
}

/// 导出全部倒计时项（不含设置）。
#[tauri::command]
#[specta::specta]
pub fn export_items(store: State<'_, Store>, path: String) -> Result<(), CommandError> {
    let data = store.read()?;
    let payload = ItemsExport {
        app: EXPORT_FORMAT,
        version: EXPORT_VERSION,
        kind: KIND_ITEMS,
        exported_at: now_stamp(),
        items: &data.items,
    };
    write_json(&path, &payload)
}

/// 导入倒计时（替换现有全部倒计时项，设置不动）。
#[tauri::command]
#[specta::specta]
pub fn import_items(
    store: State<'_, Store>,
    path: String,
) -> Result<ImportItemsResult, CommandError> {
    let envelope = read_envelope(&path)?;
    if envelope.kind != KIND_ITEMS {
        return Err(CommandError::Invalid(
            "该文件不是倒计时导出文件（kind 不匹配）".into(),
        ));
    }
    let items = envelope
        .items
        .ok_or_else(|| CommandError::Invalid("导入文件缺少 items 内容".into()))?;
    validate_items(&items)?;
    let count = items.len();
    store.mutate(move |d: &mut StoreData| -> Result<(), CommandError> {
        d.items = items;
        Ok(())
    })?;
    Ok(ImportItemsResult { items: count })
}

/// 导出设置（不含倒计时项）。
#[tauri::command]
#[specta::specta]
pub fn export_settings(store: State<'_, Store>, path: String) -> Result<(), CommandError> {
    let data = store.read()?;
    let payload = SettingsExport {
        app: EXPORT_FORMAT,
        version: EXPORT_VERSION,
        kind: KIND_SETTINGS,
        exported_at: now_stamp(),
        settings: &data.settings,
    };
    write_json(&path, &payload)
}

/// 导入设置（替换现有设置，倒计时项不动）。
#[tauri::command]
#[specta::specta]
pub fn import_settings(
    app: AppHandle,
    store: State<'_, Store>,
    path: String,
) -> Result<(), CommandError> {
    let envelope = read_envelope(&path)?;
    if envelope.kind != KIND_SETTINGS {
        return Err(CommandError::Invalid(
            "该文件不是设置导出文件（kind 不匹配）".into(),
        ));
    }
    let mut settings = envelope
        .settings
        .ok_or_else(|| CommandError::Invalid("导入文件缺少 settings 内容".into()))?;
    settings = settings.normalized();
    // 与 set_settings 同序：先校验/注册热键，再落盘；落盘失败回滚热键注册
    settings.hotkey = hotkey::canonicalize(settings.hotkey.as_deref())?;
    let prev = store.read()?.settings.hotkey;
    hotkey::apply(&app, settings.hotkey.as_deref())?;
    let saved = store.mutate(move |d: &mut StoreData| -> Result<(), CommandError> {
        d.settings = settings;
        Ok(())
    });
    if let Err(e) = saved {
        if let Err(back) = hotkey::apply(&app, prev.as_deref()) {
            log_warn!("全局热键回滚失败：{back}");
        }
        return Err(e);
    }
    Ok(())
}

fn write_json<T: serde::Serialize>(path: &str, payload: &T) -> Result<(), CommandError> {
    let json =
        serde_json::to_string_pretty(payload).map_err(|e| CommandError::Store(e.to_string()))?;
    fs::write(path, json).map_err(|e| CommandError::Message(format!("写入导出文件失败：{e}")))?;
    Ok(())
}

#[cfg(test)]
#[path = "data.test.rs"]
mod tests;
