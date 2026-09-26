//! 全量导出/导入：倒计时项 + 设置 打包为单个 JSON 文件，供备份与迁移。

use std::fs;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::domain::error::CommandError;
use crate::domain::model::{CountdownItem, Settings};
use crate::infra::store::{Store, StoreData};

const EXPORT_FORMAT: &str = "cdown-export";
const EXPORT_VERSION: u32 = 1;

#[derive(Debug, Serialize, specta::Type)]
struct ExportPayload<'a> {
    app: &'static str,
    version: u32,
    exported_at: String,
    items: &'a [CountdownItem],
    settings: &'a Settings,
}

#[derive(Debug, Deserialize)]
struct ImportPayload {
    #[serde(default)]
    items: Vec<CountdownItem>,
    #[serde(default)]
    settings: Option<Settings>,
}

#[derive(Debug, Serialize, specta::Type)]
pub struct ImportResult {
    /// 导入的倒计时条数
    pub items: usize,
}

/// 全量导出：当前全部倒计时项与设置写入 path（JSON，pretty）。
#[tauri::command]
#[specta::specta]
pub fn export_data(store: State<'_, Store>, path: String) -> Result<(), CommandError> {
    let data = store.read()?;
    let payload = ExportPayload {
        app: EXPORT_FORMAT,
        version: EXPORT_VERSION,
        exported_at: chrono::Local::now().to_rfc3339(),
        items: &data.items,
        settings: &data.settings,
    };
    let json =
        serde_json::to_string_pretty(&payload).map_err(|e| CommandError::Store(e.to_string()))?;
    fs::write(&path, json).map_err(|e| CommandError::Message(format!("写入导出文件失败：{e}")))?;
    Ok(())
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

/// 全量导入：替换语义（现有倒计时与设置整体被文件内容覆盖）。
#[tauri::command]
#[specta::specta]
pub fn import_data(store: State<'_, Store>, path: String) -> Result<ImportResult, CommandError> {
    let raw = fs::read_to_string(&path)
        .map_err(|e| CommandError::Message(format!("读取导入文件失败：{e}")))?;
    let payload: ImportPayload = serde_json::from_str(&raw)
        .map_err(|e| CommandError::Invalid(format!("导入文件不是有效的 cdown 备份：{e}")))?;
    validate_items(&payload.items)?;

    let mut settings = payload.settings.unwrap_or_default();
    settings.red_threshold_days = settings.red_threshold_days.min(365);
    settings.column_widths = settings.column_widths.sanitized();

    let count = payload.items.len();
    store.mutate(move |d: &mut StoreData| -> Result<(), CommandError> {
        d.items = payload.items;
        d.settings = settings;
        Ok(())
    })?;
    Ok(ImportResult { items: count })
}

#[cfg(test)]
#[path = "data.test.rs"]
mod tests;
