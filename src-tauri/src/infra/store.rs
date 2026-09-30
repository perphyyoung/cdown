//! JSON 文件存储：`<app_config_dir>/cdown.json`，一把 Mutex 串行化读写，
//! 写入走 临时文件 + rename 原子替换。数据量小，不引数据库。

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::domain::model::{ColumnWidths, CountdownItem, Settings};
use crate::log_warn;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StoreData {
    #[serde(default)]
    pub items: Vec<CountdownItem>,
    #[serde(default)]
    pub settings: Settings,
    /// 表格四列宽度：本机表格布局状态，不属于「设置」，不随设置备份/还原流转
    #[serde(default)]
    pub column_widths: ColumnWidths,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("序列化错误: {0}")]
    Serde(#[from] serde_json::Error),
}

/// 数据目录基准（cdown.json 所在目录）：
/// - 环境变量 `CDOWN_DATA_DIR` 优先（为 e2e/多实例隔离预留，非空才生效）；
/// - 开发环境（debug）使用项目根下的 `cdown-data`（经 CARGO_MANIFEST_DIR 编译期
///   定位，不依赖进程工作目录——tauri CLI 以 src-tauri 为 cwd 启动 exe）；
/// - 部署环境使用应用配置目录（正式数据位置不变），与 dev 天然分离。
pub fn data_dir(app: &tauri::AppHandle) -> PathBuf {
    use tauri::Manager;
    if let Ok(dir) = std::env::var("CDOWN_DATA_DIR") {
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    if cfg!(debug_assertions) {
        project_root().join("cdown-data")
    } else {
        app.path()
            .app_config_dir()
            .expect("failed to resolve app config dir")
    }
}

/// 项目根目录（src-tauri 的上级），经 CARGO_MANIFEST_DIR 编译期定位。
fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

pub struct Store {
    path: PathBuf,
    lock: Mutex<()>,
}

impl Store {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            lock: Mutex::new(()),
        }
    }

    pub fn read(&self) -> Result<StoreData, StoreError> {
        let _g = self.lock.lock().unwrap();
        self.read_locked()
    }

    /// 读取 → 修改 → 原子写回，全程持锁。闭包可用自有错误类型，
    /// 只需能从 StoreError 转换（如命令层的 CommandError）。
    pub fn mutate<F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: FnOnce(&mut StoreData) -> Result<T, E>,
        E: From<StoreError>,
    {
        let _g = self.lock.lock().unwrap();
        let mut data = self.read_locked().map_err(E::from)?;
        let out = f(&mut data)?;
        self.save_locked(&data).map_err(E::from)?;
        Ok(out)
    }

    fn read_locked(&self) -> Result<StoreData, StoreError> {
        if !self.path.exists() {
            return Ok(StoreData::default());
        }
        let raw = fs::read_to_string(&self.path)?;
        let parsed: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => {
                self.backup_corrupt(&e);
                return Ok(StoreData::default());
            }
        };
        // 一次性迁移：旧版列宽寄存在 settings.column_widths，提升为顶层字段
        let (value, migrated) = migrate_column_widths(parsed);
        let data: StoreData = match serde_json::from_value(value) {
            Ok(d) => d,
            Err(e) => {
                self.backup_corrupt(&e);
                return Ok(StoreData::default());
            }
        };
        if migrated {
            // 迁移成功立即回写新格式；失败不影响本次使用，下次启动再迁
            if let Err(e) = self.save_locked(&data) {
                log_warn!("列宽迁移后回写失败：{e}");
            }
        }
        Ok(data)
    }

    /// 备份解析失败的文件，避免被下次写入覆盖掉现场
    fn backup_corrupt(&self, e: &serde_json::Error) {
        let bak = self.path.with_extension("json.bak");
        let _ = fs::rename(&self.path, &bak);
        log_warn!("cdown.json 解析失败（已备份到 {}）：{e}", bak.display());
    }

    fn save_locked(&self, data: &StoreData) -> Result<(), StoreError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(data)?)?;
        // Windows 上 std::fs::rename 走 MOVEFILE_REPLACE_EXISTING，可覆盖已存在文件
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

/// 旧格式迁移：列宽从 `settings.column_widths` 提升到顶层 `column_widths`。
/// 顶层已存在列宽、或 settings 内没有该字段时均不改动。返回 (新值, 是否发生迁移)。
fn migrate_column_widths(mut root: serde_json::Value) -> (serde_json::Value, bool) {
    use serde_json::Value;
    let mut migrated = false;
    if let Value::Object(map) = &mut root {
        if !map.contains_key("column_widths") {
            if let Some(Value::Object(settings)) = map.get_mut("settings") {
                if let Some(w) = settings.remove("column_widths") {
                    map.insert("column_widths".to_string(), w);
                    migrated = true;
                }
            }
        }
    }
    (root, migrated)
}

#[cfg(test)]
#[path = "store.test.rs"]
mod tests;
