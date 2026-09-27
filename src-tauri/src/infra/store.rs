//! JSON 文件存储：`<app_config_dir>/cdown.json`，一把 Mutex 串行化读写，
//! 写入走 临时文件 + rename 原子替换。数据量小，不引数据库。

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::domain::model::{CountdownItem, Settings};
use crate::log_warn;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StoreData {
    #[serde(default)]
    pub items: Vec<CountdownItem>,
    #[serde(default)]
    pub settings: Settings,
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
        match serde_json::from_str(&raw) {
            Ok(data) => Ok(data),
            Err(e) => {
                // 备份损坏文件，避免被下次写入覆盖掉现场
                let bak = self.path.with_extension("json.bak");
                let _ = fs::rename(&self.path, &bak);
                log_warn!("cdown.json 解析失败（已备份到 {}）：{e}", bak.display());
                Ok(StoreData::default())
            }
        }
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

#[cfg(test)]
#[path = "store.test.rs"]
mod tests;
