//! 字体中文名映射：数据目录下的 `font-family-map.toml`（英文族名 → 中文显示名）。
//! 仅用于设置页下拉的显示文案，不参与任何渲染决策；文件缺失时从随程序内嵌的模板
//! 原样生成，用户可自行增删。逐行解析（注释/空行/不规范行跳过），读取失败回退模板。
//!
//! 默认映射**只存在于模板文件**（`resources/font-family-map-template.toml`），代码无硬编码
//! 条目：新增默认映射改模板即可，其他项目复用直接复制该模板。用 `include_str!` 编译期
//! 内嵌，dev/release/e2e 拿到同一份字节，无需按运行环境探测模板路径。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::log_warn;

pub const MAP_FILE_NAME: &str = "font-family-map.toml";

/// 随程序分发的默认映射模板（含注释），编译期内嵌
const TEMPLATE: &str = include_str!("../../resources/font-family-map-template.toml");

/// 读取映射：文件不存在时把模板原样写入数据目录；读失败回退模板内容。
pub fn load_or_create(dir: &Path) -> BTreeMap<String, String> {
    let path = dir.join(MAP_FILE_NAME);
    if !path.exists() {
        if let Err(e) = fs::write(&path, TEMPLATE) {
            log_warn!("写入 {MAP_FILE_NAME} 失败：{e}");
        }
    }
    match fs::read_to_string(&path) {
        Ok(content) => parse(&content),
        Err(e) => {
            log_warn!("读取 {MAP_FILE_NAME} 失败：{e}，回退内置模板");
            parse(TEMPLATE)
        }
    }
}

/// 模板解析结果（测试用：路径只在此模块出现一次）
#[cfg(test)]
pub(crate) fn template_map() -> BTreeMap<String, String> {
    parse(TEMPLATE)
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

#[cfg(test)]
#[path = "font_map.test.rs"]
mod tests;
