use super::{CountdownItem, Settings};

#[test]
fn settings_default_is_three_days() {
    assert_eq!(Settings::default().red_threshold_days, 3);
    // 反序列化缺字段时也应回落到默认值（旧版/手改的 cdown.json 兼容）
    let s: Settings = serde_json::from_str("{}").unwrap();
    assert_eq!(s.red_threshold_days, 3);
}

#[test]
fn item_serializes_with_expected_field_names() {
    let item = CountdownItem {
        id: "it-1".into(),
        title: "测试".into(),
        target_date: "2030-01-01".into(),
        note: None,
        created_at: "2026-01-01T00:00:00+08:00".into(),
    };
    let raw = serde_json::to_string(&item).unwrap();
    assert!(raw.contains("\"target_date\""));
    assert!(raw.contains("\"created_at\""));
}
