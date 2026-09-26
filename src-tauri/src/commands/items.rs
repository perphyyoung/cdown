//! 倒计时项命令：增删改查，全部落 JSON 存储。

use tauri::State;

use crate::domain::error::CommandError;
use crate::domain::model::CountdownItem;
use crate::infra::store::{Store, StoreData};

/// 生成唯一 id：纳秒时间戳 + 进程内序号，避免引入 uuid 依赖。
fn new_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    format!("it-{nanos:032x}-{seq:04x}")
}

fn validate_title_and_date(title: &str, target_date: &str) -> Result<(), CommandError> {
    if title.trim().is_empty() {
        return Err(CommandError::Invalid("名称不能为空".into()));
    }
    chrono::NaiveDate::parse_from_str(target_date, "%Y-%m-%d").map_err(|_| {
        CommandError::Invalid(format!("目标日期格式应为 YYYY-MM-DD：{target_date}"))
    })?;
    Ok(())
}

/// 列出全部倒计时项（存储顺序返回，排序展示由前端负责）。
#[tauri::command]
#[specta::specta]
pub fn list_items(store: State<'_, Store>) -> Result<Vec<CountdownItem>, CommandError> {
    Ok(store.read()?.items)
}

#[tauri::command]
#[specta::specta]
pub fn add_item(
    store: State<'_, Store>,
    title: String,
    target_date: String,
    note: Option<String>,
) -> Result<CountdownItem, CommandError> {
    validate_title_and_date(&title, &target_date)?;
    let item = CountdownItem {
        id: new_id(),
        title: title.trim().to_string(),
        target_date,
        note,
        created_at: chrono::Local::now().to_rfc3339(),
    };
    store.mutate(|d: &mut StoreData| -> Result<(), CommandError> {
        d.items.push(item.clone());
        Ok(())
    })?;
    Ok(item)
}

#[tauri::command]
#[specta::specta]
pub fn update_item(
    store: State<'_, Store>,
    id: String,
    title: String,
    target_date: String,
    note: Option<String>,
) -> Result<(), CommandError> {
    validate_title_and_date(&title, &target_date)?;
    store.mutate(|d: &mut StoreData| -> Result<(), CommandError> {
        let item = d
            .items
            .iter_mut()
            .find(|it| it.id == id)
            .ok_or_else(|| CommandError::NotFound(format!("未找到倒计时项 {id}")))?;
        item.title = title.trim().to_string();
        item.target_date = target_date;
        item.note = note;
        Ok(())
    })?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn delete_item(store: State<'_, Store>, id: String) -> Result<(), CommandError> {
    store.mutate(|d: &mut StoreData| -> Result<(), CommandError> {
        let before = d.items.len();
        d.items.retain(|it| it.id != id);
        if d.items.len() == before {
            return Err(CommandError::NotFound(format!("未找到倒计时项 {id}")));
        }
        Ok(())
    })?;
    Ok(())
}

#[cfg(test)]
#[path = "items.test.rs"]
mod tests;
