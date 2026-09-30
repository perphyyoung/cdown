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

/// 主窗口背景透明度下限：低于此值窗口几乎不可见，无实际意义
pub const MIN_OPACITY: u8 = 10;

fn default_opacity() -> u8 {
    100
}

/// 主窗口默认背景色（slate-900）
pub const DEFAULT_BACKGROUND_COLOR: &str = "#0f172a";

fn default_background_color() -> String {
    DEFAULT_BACKGROUND_COLOR.into()
}

/// 表格基准字号默认值（px）：名称/备注列
const DEFAULT_FONT_SIZE: u8 = 14;
/// 表格字号上下限（px）
pub const MIN_FONT_SIZE: u8 = 10;
pub const MAX_FONT_SIZE: u8 = 20;

fn default_font_size() -> u8 {
    DEFAULT_FONT_SIZE
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
    /// 主窗口置顶（标题栏图钉切换）；旧数据缺字段按默认置顶处理
    #[serde(default = "default_true")]
    pub always_on_top: bool,
    /// 全局热键 accelerator（如 `Ctrl+Alt+C`）唤起主窗口；None = 关闭热键。
    /// 字段缺失（旧数据）回落默认键，显式 null 表示用户关闭 —— serde 只在缺失时用 default。
    #[serde(default = "default_hotkey")]
    pub hotkey: Option<String>,
    /// 主窗口背景透明度（%），10–100；旧数据缺字段按不透明处理
    #[serde(default = "default_opacity")]
    pub background_opacity: u8,
    /// 主窗口背景色 `#RRGGBB`；非法值回落默认色
    #[serde(default = "default_background_color")]
    pub background_color: String,
    /// 全局字体家族：纯族名（如 `Microsoft YaHei`），CSS 拼接由前端完成；
    /// 空串 = 跟随系统默认栈（旧数据缺字段即落这里，行为不变）
    #[serde(default)]
    pub font_family: String,
    /// 主界面表格基准字号（px），10–20：名称/备注列取基准值，
    /// 表头/倒计时/目标日期列取基准 - 2；旧数据缺字段按默认 14 处理
    #[serde(default = "default_font_size")]
    pub font_size: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            levels: default_levels(),
            always_on_top: true,
            hotkey: default_hotkey(),
            background_opacity: default_opacity(),
            background_color: default_background_color(),
            font_family: String::new(),
            font_size: default_font_size(),
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
        // 透明度：夹到 10–100
        self.background_opacity = self.background_opacity.clamp(MIN_OPACITY, 100);
        // 背景色：非法回落默认色
        if !is_hex_color(&self.background_color) {
            self.background_color = DEFAULT_BACKGROUND_COLOR.into();
        }
        // 字体家族：空串 = 跟随系统；只接受纯族名（中英文/数字/空格与 . _ -），
        // 与前端 sanitizeFontFamily 的合法字符集一致；含逗号、引号、分号等或超长一律回落空串
        let font_family = self.font_family.trim().to_string();
        self.font_family = if is_font_family_value(&font_family) {
            font_family
        } else {
            String::new()
        };
        // 字号：夹到 10–20，旧数据 0 或缺字段回落默认 14
        self.font_size = if self.font_size == 0 {
            DEFAULT_FONT_SIZE
        } else {
            self.font_size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE)
        };
        self
    }
}

/// 家族名长度上限，与前端 sanitizeFontFamily 的 FONT_FAMILY_MAX_LEN 一致
const MAX_FONT_FAMILY_LEN: usize = 64;

fn is_font_family_value(s: &str) -> bool {
    s.chars().count() <= MAX_FONT_FAMILY_LEN
        && s.chars()
            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '.' | '_' | '-'))
}

fn is_hex_color(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 7 && b[0] == b'#' && b[1..].iter().all(u8::is_ascii_hexdigit)
}

#[cfg(test)]
#[path = "model.test.rs"]
mod tests;
