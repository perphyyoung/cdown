//! 字体中文名映射：数据目录下的 `font-family-map.toml`（英文族名 → 中文显示名）。
//! 仅用于设置页下拉的显示文案，不参与任何渲染决策；文件缺失时写入默认模板，
//! 用户可自行增删。逐行解析（注释/空行/不规范行跳过），读写失败一律回落内置默认。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::log_warn;

pub const MAP_FILE_NAME: &str = "font-family-map.toml";

/// 默认映射：首次生成模板时写入（与参考项目 chat-manager 保持一致）
pub const DEFAULT_MAP: &[(&str, &str)] = &[
    ("Microsoft YaHei", "微软雅黑"),
    ("Microsoft YaHei UI", "微软雅黑 UI"),
    ("PingFang SC", "苹方"),
    ("Hiragino Sans GB", "冬青黑体"),
    ("Noto Sans SC", "思源黑体"),
    ("Noto Serif SC", "思源宋体"),
    ("Source Han Sans SC", "思源黑体"),
    ("Source Han Serif SC", "思源宋体"),
    ("SimSun", "宋体"),
    ("NSimSun", "新宋体"),
    ("SimHei", "黑体"),
    ("KaiTi", "楷体"),
    ("FangSong", "仿宋"),
    ("DengXian", "等线"),
    ("Sarasa Mono SC", "更纱黑体"),
    ("Sarasa UI SC", "更纱黑体"),
    ("Sarasa Term SC", "更纱黑体"),
    ("Sarasa Gothic SC", "更纱黑体"),
];

/// 读取映射：文件不存在时先写默认模板。读失败回落默认映射。
pub fn load_or_create(dir: &Path) -> BTreeMap<String, String> {
    let path = dir.join(MAP_FILE_NAME);
    if !path.exists() {
        if let Err(e) = fs::write(&path, default_template()) {
            log_warn!("写入 {MAP_FILE_NAME} 失败：{e}");
        }
    }
    match fs::read_to_string(&path) {
        Ok(content) => parse(&content),
        Err(e) => {
            log_warn!("读取 {MAP_FILE_NAME} 失败：{e}");
            default_map()
        }
    }
}

/// 逐行解析 `"英文族名" = "中文名"`：注释、空行与不规范行跳过，单个坏行不影响其余行。
fn parse(content: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().trim_matches('"').trim();
        let value = value.trim().trim_matches('"').trim();
        if key.is_empty() || value.is_empty() {
            continue;
        }
        map.insert(key.to_string(), value.to_string());
    }
    map
}

fn default_map() -> BTreeMap<String, String> {
    DEFAULT_MAP
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

fn default_template() -> String {
    let mut s = String::from(
        "# 字体英文族名 → 中文显示名映射\n\
         # 每行一个映射，语法：\"英文族名\" = \"中文名\"\n\
         # 不规范的行会被跳过，不影响其他行\n",
    );
    for (k, v) in DEFAULT_MAP {
        s.push_str(&format!("\"{k}\" = \"{v}\"\n"));
    }
    s
}

#[cfg(test)]
#[path = "font_map.test.rs"]
mod tests;
