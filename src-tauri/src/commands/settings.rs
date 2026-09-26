//! 设置命令：临近变红阈值（天）与表格列宽。

use tauri::State;

use crate::domain::error::CommandError;
use crate::domain::model::Settings;
use crate::infra::store::{Store, StoreData};

#[tauri::command]
#[specta::specta]
pub fn get_settings(store: State<'_, Store>) -> Result<Settings, CommandError> {
    Ok(store.read()?.settings)
}

#[tauri::command]
#[specta::specta]
pub fn set_settings(store: State<'_, Store>, settings: Settings) -> Result<Settings, CommandError> {
    let settings = Settings {
        red_threshold_days: settings.red_threshold_days,
        column_widths: settings.column_widths.sanitized(),
    };
    if settings.red_threshold_days > 365 {
        return Err(CommandError::Invalid("阈值不能超过 365 天".into()));
    }
    store.mutate(|d: &mut StoreData| -> Result<(), CommandError> {
        d.settings = settings.clone();
        Ok(())
    })?;
    Ok(settings)
}
