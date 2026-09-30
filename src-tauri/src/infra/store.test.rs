use super::{Store, StoreData, StoreError};
use crate::domain::model::{ColumnWidths, CountdownItem, Settings, UrgencyLevel};
use std::fs;

fn temp_store(tag: &str) -> Store {
    let dir = std::env::temp_dir().join(format!("cdown-test-{}-{tag}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    Store::new(dir.join("cdown.json"))
}

fn sample_item(id: &str) -> CountdownItem {
    CountdownItem {
        id: id.into(),
        title: "测试项".into(),
        target_date: "2030-01-01".into(),
        note: None,
        created_at: "2026-01-01T00:00:00+08:00".into(),
    }
}

#[test]
fn missing_file_yields_default() {
    let store = temp_store("missing");
    let data = store.read().unwrap();
    assert!(data.items.is_empty());
    assert_eq!(data.settings.levels, Settings::default().levels);
}

#[test]
fn mutate_roundtrip_persists_items() {
    let store = temp_store("roundtrip");
    let item = sample_item("it-1");
    store
        .mutate(|d: &mut StoreData| -> Result<(), StoreError> {
            d.items.push(item.clone());
            Ok(())
        })
        .unwrap();
    let data = store.read().unwrap();
    assert_eq!(data.items.len(), 1);
    assert_eq!(data.items[0].id, "it-1");
    // Settings 缺省字段回落默认值
    assert_eq!(data.settings.levels, Settings::default().levels);
}

#[test]
fn corrupt_file_backs_up_and_falls_back_to_default() {
    let store = temp_store("corrupt");
    fs::write(store.path.clone(), "{ not json").unwrap();
    let data = store.read().unwrap();
    assert!(data.items.is_empty());
    assert_eq!(data.settings.levels, Settings::default().levels);
    // 现场保留为 .json.bak，且原文件已被移走（下次写入不会覆盖损坏现场）
    let bak = store.path.with_extension("json.bak");
    assert!(bak.exists());
    assert!(!store.path.exists());
}

#[test]
fn save_then_reload_keeps_settings() {
    let store = temp_store("settings");
    store
        .mutate(|d: &mut StoreData| -> Result<(), StoreError> {
            d.settings = Settings {
                levels: vec![UrgencyLevel {
                    threshold_days: 7,
                    color: "#a78bfa".into(),
                }],
                always_on_top: false,
                hotkey: Some("Ctrl+Alt+K".into()),
                background_opacity: 100,
                background_color: "#0f172a".into(),
                font_family: "SimSun".into(),
                font_size: 16,
            };
            Ok(())
        })
        .unwrap();
    let data = store.read().unwrap();
    assert_eq!(data.settings.levels.len(), 1);
    assert_eq!(data.settings.levels[0].threshold_days, 7);
    // always_on_top 随设置整体持久化
    assert!(!data.settings.always_on_top);
    // 热键也随设置整体持久化
    assert_eq!(data.settings.hotkey.as_deref(), Some("Ctrl+Alt+K"));
    // 字号随设置整体持久化
    assert_eq!(data.settings.font_size, 16);
}

#[test]
fn legacy_column_widths_migrated_to_top_level_and_rewritten() {
    // 旧版列宽寄存在 settings.column_widths：读出后提升到顶层并立即回写新格式
    let store = temp_store("migrate-widths");
    let legacy = serde_json::json!({
        "items": [],
        "settings": {
            "levels": [],
            "column_widths": { "name": 200, "target": 120, "countdown": 100, "note": 110 }
        }
    });
    fs::write(store.path.clone(), serde_json::to_vec(&legacy).unwrap()).unwrap();

    let want = ColumnWidths {
        name: 200,
        target: 120,
        countdown: 100,
        note: 110,
    };
    let data = store.read().unwrap();
    assert_eq!(data.column_widths, want);

    // 回写文件：列宽已在顶层，settings 内不再含该字段
    let rewritten: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&store.path).unwrap()).unwrap();
    assert_eq!(rewritten["column_widths"]["name"], 200);
    assert!(rewritten["settings"].get("column_widths").is_none());

    // 再读幂等，值不变
    assert_eq!(store.read().unwrap().column_widths, want);
}

#[test]
fn replacing_settings_keeps_top_level_widths() {
    // set_settings/reset_settings 只换 settings，顶层列宽必须原样保留
    let store = temp_store("widths-untouched");
    store
        .mutate(|d: &mut StoreData| -> Result<(), StoreError> {
            d.column_widths = ColumnWidths {
                name: 200,
                target: 120,
                countdown: 100,
                note: 110,
            };
            d.settings.always_on_top = false;
            Ok(())
        })
        .unwrap();
    store
        .mutate(|d: &mut StoreData| -> Result<(), StoreError> {
            d.settings = Settings::default();
            Ok(())
        })
        .unwrap();
    let data = store.read().unwrap();
    assert!(data.settings.always_on_top);
    assert_eq!(
        data.column_widths,
        ColumnWidths {
            name: 200,
            target: 120,
            countdown: 100,
            note: 110,
        }
    );
}
