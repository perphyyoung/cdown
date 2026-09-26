//! 领域模型：倒计时项与设置。日期统一用 `YYYY-MM-DD` 字符串承载（精度到天），
//! Rust 侧需要计算/校验时再经 chrono::NaiveDate 解析。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, specta::Type)]
pub struct CountdownItem {
    pub id: String,
    pub title: String,
    /// 目标日期，`YYYY-MM-DD`
    pub target_date: String,
    pub note: Option<String>,
    /// 创建时间，RFC3339（仅记录，不参与倒计时显示）
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, specta::Type)]
pub struct Settings {
    /// 临近阈值（天）：剩余天数 ≤ 该值时前端标红；默认 3
    #[serde(default = "default_red_threshold_days")]
    pub red_threshold_days: u32,
}

fn default_red_threshold_days() -> u32 {
    3
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            red_threshold_days: 3,
        }
    }
}

#[cfg(test)]
#[path = "model.test.rs"]
mod tests;
