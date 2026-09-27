use super::{enabled, level_from_str, parse_config_text, set_min_level, Level};

#[test]
fn parse_config_reads_quoted_value() {
    let text = "# 注释
dev-log-level = \"debug\"
";
    let cfg = parse_config_text(text);
    assert_eq!(cfg.dev_log_level.as_deref(), Some("debug"));
    assert_eq!(cfg.release_log_level, None);
}

#[test]
fn parse_config_ignores_comments_and_other_keys() {
    let text = "release-log-level = \"warn\"
# dev-log-level = \"debug\"
";
    let cfg = parse_config_text(text);
    assert_eq!(cfg.dev_log_level, None);
    assert_eq!(cfg.release_log_level.as_deref(), Some("warn"));
}

#[test]
fn parse_config_invalid_toml_falls_back_to_default() {
    let cfg = parse_config_text("not toml {{{");
    assert_eq!(cfg.dev_log_level, None);
    assert_eq!(cfg.release_log_level, None);
}

#[test]
fn level_from_str_is_case_insensitive_and_rejects_unknown() {
    assert_eq!(level_from_str("DEBUG"), Some(Level::Debug));
    assert_eq!(level_from_str("Warn"), Some(Level::Warn));
    assert_eq!(level_from_str("verbose"), None);
}

#[test]
fn enabled_filters_below_min_level() {
    set_min_level(Level::Warn);
    assert!(!enabled(Level::Debug));
    assert!(!enabled(Level::Info));
    assert!(enabled(Level::Warn));
    assert!(enabled(Level::Error));
    // 还原为 debug，避免影响其它用例（全局开关）
    set_min_level(Level::Debug);
}
