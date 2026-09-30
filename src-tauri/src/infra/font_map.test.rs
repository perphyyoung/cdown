use super::{load_or_create, parse, template_map, MAP_FILE_NAME};

#[test]
fn parse_skips_comments_blank_and_malformed_lines() {
    let map = parse(
        "# 注释行\n\n\"Microsoft YaHei\" = \"微软雅黑\"\n没有等号的行\n= \"无键\"\n\"Empty\" = \"\"\n",
    );
    assert_eq!(map.len(), 1);
    assert_eq!(
        map.get("Microsoft YaHei").map(String::as_str),
        Some("微软雅黑")
    );
}

#[test]
fn parse_tolerates_unquoted_keys_and_extra_spaces() {
    let map = parse("  SimSun   =   \"宋体\"  \n\"KaiTi\"=\"楷体\"\n");
    assert_eq!(map.get("SimSun").map(String::as_str), Some("宋体"));
    assert_eq!(map.get("KaiTi").map(String::as_str), Some("楷体"));
}

#[test]
fn missing_file_writes_template_and_returns_default_map() {
    let dir = temp_dir("missing");
    let map = load_or_create(&dir);
    let expected = template_map();
    assert_eq!(map.len(), expected.len());
    assert_eq!(
        map.get("Microsoft YaHei").map(String::as_str),
        Some("微软雅黑")
    );
    // 模板已原样落盘，内容解析回来与内置模板一致
    let content = std::fs::read_to_string(dir.join(MAP_FILE_NAME)).unwrap();
    assert_eq!(parse(&content).len(), expected.len());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn existing_file_wins_over_template() {
    let dir = temp_dir("user");
    std::fs::write(dir.join(MAP_FILE_NAME), "\"My Font\" = \"我的字体\"\n").unwrap();
    let map = load_or_create(&dir);
    assert_eq!(map.len(), 1);
    assert_eq!(map.get("My Font").map(String::as_str), Some("我的字体"));
    std::fs::remove_dir_all(&dir).unwrap();
}

/// 每个用例独立的临时目录（已清空重建）
fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("cdown-font-map-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
