use super::{
    ColumnWidths, CountdownItem, Settings, UrgencyLevel, DEFAULT_BACKGROUND_COLOR, DEFAULT_HOTKEY,
    MAX_FONT_SIZE, MIN_FONT_SIZE, MIN_OPACITY,
};

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
    assert_eq!(s.levels[2].threshold_days, 0);
    assert_eq!(s.levels[2].color, "#fb923c");
    assert!(Settings::default().always_on_top);
    // 反序列化缺字段时也应回落到默认值（旧版/手改的 cdown.json 兼容）
    let s: Settings = serde_json::from_str("{}").unwrap();
    assert_eq!(s.levels, Settings::default().levels);
    assert!(s.always_on_top);
    assert_eq!(s.hotkey.as_deref(), Some(DEFAULT_HOTKEY));
    // 透明度缺字段回落不透明
    assert_eq!(s.background_opacity, 100);
    // 背景色缺字段回落默认色
    assert_eq!(s.background_color, DEFAULT_BACKGROUND_COLOR);
}

#[test]
fn default_settings_survive_normalize() {
    // reset_settings 的数据源：默认值过一遍 normalized 不得漂移
    let s = Settings::default().normalized();
    let d = Settings::default();
    assert_eq!(s.levels, d.levels);
    assert_eq!(s.column_widths, d.column_widths);
    assert!(s.always_on_top);
    assert_eq!(s.hotkey.as_deref(), Some(DEFAULT_HOTKEY));
    assert_eq!(s.background_opacity, 100);
    assert_eq!(s.background_color, DEFAULT_BACKGROUND_COLOR);
    assert_eq!(s.font_family, "");
    assert_eq!(s.font_size, 14);
}

#[test]
fn hotkey_default_only_when_field_missing() {
    // 显式 null = 用户主动关闭热键，不能被默认值覆盖
    let s: Settings = serde_json::from_str("{\"hotkey\":null}").unwrap();
    assert_eq!(s.hotkey, None);
    // 自定键原样保留
    let s: Settings = serde_json::from_str("{\"hotkey\":\"Ctrl+Alt+K\"}").unwrap();
    assert_eq!(s.hotkey.as_deref(), Some("Ctrl+Alt+K"));
    // 空白串归一为 None（关闭热键）
    let s = Settings {
        hotkey: Some("   ".into()),
        ..Settings::default()
    }
    .normalized();
    assert_eq!(s.hotkey, None);
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
        always_on_top: true,
        hotkey: None,
        background_opacity: 100,
        background_color: DEFAULT_BACKGROUND_COLOR.into(),
        font_family: String::new(),
        font_size: 14,
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
        always_on_top: true,
        hotkey: None,
        background_opacity: 100,
        background_color: DEFAULT_BACKGROUND_COLOR.into(),
        font_family: String::new(),
        font_size: 14,
    }
    .normalized();
    assert_eq!(s.levels.len(), 6);
    assert_eq!(s.levels[0].threshold_days, 10); // 降序后保留阈值最大的 6 级
}

#[test]
fn background_opacity_clamps() {
    // 过低夹到下限，非法大值夹到 100
    let s = Settings {
        background_opacity: 3,
        ..Settings::default()
    }
    .normalized();
    assert_eq!(s.background_opacity, MIN_OPACITY);
    let s = Settings {
        background_opacity: 55, // 合法值原样保留
        ..Settings::default()
    }
    .normalized();
    assert_eq!(s.background_opacity, 55);
}

#[test]
fn font_family_normalized_trims_and_falls_back() {
    // 合法纯族名：去空白后原样保留
    let s = Settings {
        font_family: "  Microsoft YaHei  ".into(),
        ..Settings::default()
    }
    .normalized();
    assert_eq!(s.font_family, "Microsoft YaHei");
    // 空串 = 跟随系统
    let s = Settings {
        font_family: "   ".into(),
        ..Settings::default()
    }
    .normalized();
    assert_eq!(s.font_family, "");
    // 不是纯族名的（CSS 值、注入字符）与超长值：一律回落空串
    let too_long = "a".repeat(65);
    for bad in [
        "\"Microsoft YaHei\", sans-serif",
        "sans; color: red",
        "a { b }",
        "a\\b",
        "a\nb",
        too_long.as_str(),
    ] {
        let s = Settings {
            font_family: bad.into(),
            ..Settings::default()
        }
        .normalized();
        assert_eq!(s.font_family, "", "非法字体值 {bad} 应回落空串");
    }
}

#[test]
fn font_size_defaults_and_clamps() {
    // 默认 14
    assert_eq!(Settings::default().font_size, 14);
    // 过小/过大夹到上下限；合法值原样保留；0（旧数据显式写入）回落默认 14
    for (input, want) in [(3, MIN_FONT_SIZE), (40, MAX_FONT_SIZE), (16, 16), (0, 14)] {
        let s = Settings {
            font_size: input,
            ..Settings::default()
        }
        .normalized();
        assert_eq!(s.font_size, want, "font_size {input} 归一化应为 {want}");
    }
}

#[test]
fn background_color_falls_back_when_invalid() {
    // 合法色原样保留
    let s = Settings {
        background_color: "#123456".into(),
        ..Settings::default()
    }
    .normalized();
    assert_eq!(s.background_color, "#123456");
    // 非法色回落默认
    for bad in ["", "red", "#12345", "#12g456", "123456"] {
        let s = Settings {
            background_color: bad.into(),
            ..Settings::default()
        }
        .normalized();
        assert_eq!(
            s.background_color, DEFAULT_BACKGROUND_COLOR,
            "非法色 {bad} 应回落"
        );
    }
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
