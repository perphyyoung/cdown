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

/// 紧急度分级：剩余天数 ≤ threshold_days 时该行采用 color 显示（含当天 days = 0）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, specta::Type)]
pub struct UrgencyLevel {
    pub threshold_days: u32,
    /// 行文字颜色，`#RRGGBB`
    pub color: String,
}

const MAX_LEVELS: usize = 6;
const FALLBACK_COLOR: &str = "#fb923c";

fn default_true() -> bool {
    true
}

/// 默认全局热键（accelerator 串，格式见 global-hotkey 解析器：修饰键在前 + 一个主键）
pub const DEFAULT_HOTKEY: &str = "Ctrl+Alt+C";

fn default_hotkey() -> Option<String> {
    Some(DEFAULT_HOTKEY.into())
}

fn default_levels() -> Vec<UrgencyLevel> {
    vec![
        UrgencyLevel {
            threshold_days: 7,
            color: "#a78bfa".into(), // 紫 violet-400
        },
        UrgencyLevel {
            threshold_days: 3,
            color: "#facc15".into(), // 黄 yellow-400
        },
        UrgencyLevel {
            threshold_days: 0,
            color: "#fb923c".into(), // 橙 orange-400，仅命中当天
        },
    ]
}

/// 主窗口几何（物理像素）；由后端跟踪保存，前端设置保存不携带此字段
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, specta::Type)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, specta::Type)]
pub struct Settings {
    /// 紧急度分级：按阈值降序存储；过期固定红色，不在此列
    #[serde(default = "default_levels")]
    pub levels: Vec<UrgencyLevel>,
    /// 表格四列宽度（px）
    #[serde(default)]
    pub column_widths: ColumnWidths,
    /// 主窗口置顶（标题栏图钉切换）；旧数据缺字段按默认置顶处理
    #[serde(default = "default_true")]
    pub always_on_top: bool,
    /// 全局热键 accelerator（如 `Ctrl+Alt+C`）唤起主窗口；None = 关闭热键。
    /// 字段缺失（旧数据）回落默认键，显式 null 表示用户关闭 —— serde 只在缺失时用 default。
    #[serde(default = "default_hotkey")]
    pub hotkey: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            levels: default_levels(),
            column_widths: ColumnWidths::default(),
            always_on_top: true,
            hotkey: default_hotkey(),
        }
    }
}

impl Settings {
    /// 归一化：非法颜色回落、阈值去重、降序排序、条数上限
    pub fn normalized(mut self) -> Self {
        let mut seen: Vec<u32> = Vec::new();
        let mut levels = Vec::with_capacity(self.levels.len());
        for l in self.levels.drain(..) {
            if seen.contains(&l.threshold_days) {
                continue;
            }
            seen.push(l.threshold_days);
            let color = if is_hex_color(&l.color) {
                l.color
            } else {
                FALLBACK_COLOR.into()
            };
            levels.push(UrgencyLevel {
                threshold_days: l.threshold_days,
                color,
            });
        }
        levels.sort_by(|a, b| b.threshold_days.cmp(&a.threshold_days));
        levels.truncate(MAX_LEVELS);
        self.levels = levels;
        // 热键：仅做去空白与空串归 None（合法性由注册时的解析器判定，非法键在设置页报错）
        self.hotkey = self
            .hotkey
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        self
    }
}

fn is_hex_color(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 7 && b[0] == b'#' && b[1..].iter().all(u8::is_ascii_hexdigit)
}

#[cfg(test)]
#[path = "model.test.rs"]
mod tests;
