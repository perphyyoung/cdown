//! 表格列宽命令：列宽是随本机表格数据存在的布局状态，
//! 独立于「设置」——不参与设置备份的导入导出，也不被一键还原触碰。

use tauri::State;

use crate::domain::error::CommandError;
use crate::domain::model::ColumnWidths;
use crate::infra::store::{Store, StoreData};

#[tauri::command]
#[specta::specta]
pub fn get_column_widths(store: State<'_, Store>) -> Result<ColumnWidths, CommandError> {
    Ok(store.read()?.column_widths)
}

/// 拖拽列宽落盘；落盘前统一夹取（24–400px），返回夹取后的值。
#[tauri::command]
#[specta::specta]
pub fn set_column_widths(
    store: State<'_, Store>,
    widths: ColumnWidths,
) -> Result<ColumnWidths, CommandError> {
    let widths = widths.sanitized();
    store.mutate(|d: &mut StoreData| {
        d.column_widths = widths;
        Ok(widths)
    })
}
