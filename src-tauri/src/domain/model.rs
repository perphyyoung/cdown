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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, specta::Type)]
#[serde(default)]
pub struct ColumnWidths {
    /// 各列宽度（px），可拖拽调整；默认值适配 320px 初始窗口，clamp 范围见 sanitized()
    pub name: u32,
    pub target: u32,
    /// 倒计时列
    pub countdown: u32,
    pub note: u32,
}

impl Default for ColumnWidths {
    fn default() -> Self {
        Self {
            name: 92,
            target: 70,
            countdown: 52,
            note: 50,
        }
    }
}

impl ColumnWidths {
    pub fn sanitized(&self) -> Self {
        let clamp = |v: u32| v.clamp(24, 400);
        Self {
            name: clamp(self.name),
            target: clamp(self.target),
            countdown: clamp(self.countdown),
            note: clamp(self.note),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, specta::Type)]
pub struct Settings {
    /// 临近阈值（天）：剩余天数 ≤ 该值时前端标红；默认 3
    #[serde(default = "default_red_threshold_days")]
    pub red_threshold_days: u32,
    /// 表格四列宽度（px）
    #[serde(default)]
    pub column_widths: ColumnWidths,
}

fn default_red_threshold_days() -> u32 {
    3
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            red_threshold_days: 3,
            column_widths: ColumnWidths::default(),
        }
    }
}

#[cfg(test)]
#[path = "model.test.rs"]
mod tests;
