use super::{ColumnWidths, CountdownItem, Settings, UrgencyLevel};

fn level(days: u32, color: &str) -> UrgencyLevel {
    UrgencyLevel {
        threshold_days: days,
        color: color.into(),
    }
}

#[test]
fn settings_default_is_three_levels() {
    let s = Settings::default();
    assert_eq!(s.levels.len(), 3);
    assert_eq!(s.levels[0].threshold_days, 7);
    assert_eq!(s.levels[0].color, "#a78bfa");
    assert_eq!(s.levels[1].threshold_days, 3);
    assert_eq!(s.levels[1].color, "#facc15");
    assert_eq!(s.levels[2].threshold_days, 1);
    assert_eq!(s.levels[2].color, "#fb923c");
    // 反序列化缺字段时也应回落到默认值（旧版/手改的 cdown.json 兼容）
    let s: Settings = serde_json::from_str("{}").unwrap();
    assert_eq!(s.levels, Settings::default().levels);
}

#[test]
fn column_widths_fill_defaults_when_missing() {
    // 旧版 settings 只有阈值字段，column_widths 缺失时整体回落默认值
    let s: Settings = serde_json::from_str("{\"levels\":[]}").unwrap();
    assert!(s.levels.is_empty());
    assert_eq!(s.column_widths, ColumnWidths::default());
    assert!(ColumnWidths::default().name > 0);
}

#[test]
fn column_widths_sanitized_clamps() {
    let w = ColumnWidths {
        name: 10,
        target: 100,
        countdown: 1000,
        note: 100,
    }
    .sanitized();
    assert_eq!(w.name, 24);
    assert_eq!(w.countdown, 400);
    assert_eq!(w.target, 100);
}

#[test]
fn settings_normalized_sorts_dedupes_and_fixes_colors() {
    let s = Settings {
        levels: vec![
            level(3, "orange"),  // 非法颜色 → 回落
            level(7, "#a78bfa"), // 与后面的 7 重复 → 去重
            level(7, "#fb923c"),
            level(10, "#12g45z"), // 非法颜色 → 回落
            level(1, "#ffffff"),
        ],
        column_widths: ColumnWidths::default(),
    }
    .normalized();
    let thresholds: Vec<u32> = s.levels.iter().map(|l| l.threshold_days).collect();
    assert_eq!(thresholds, vec![10, 7, 3, 1]);
    assert_eq!(s.levels[0].color, "#fb923c"); // 非法回落默认橙
    assert_eq!(s.levels[1].color, "#a78bfa"); // 重复阈值保留先出现的颜色
    assert_eq!(s.levels[2].color, "#fb923c"); // 非法回落默认橙
}

#[test]
fn settings_normalized_caps_levels() {
    let mut levels = Vec::new();
    for i in 0..10_u32 {
        levels.push(level(i + 1, "#ffffff"));
    }
    let s = Settings {
        levels,
        column_widths: ColumnWidths::default(),
    }
    .normalized();
    assert_eq!(s.levels.len(), 6);
    assert_eq!(s.levels[0].threshold_days, 10); // 降序后保留阈值最大的 6 级
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
