use super::{validate_items, ImportEnvelope, KIND_ITEMS, KIND_SETTINGS};
use crate::domain::model::CountdownItem;

fn item(title: &str, date: &str) -> CountdownItem {
    CountdownItem {
        id: "it-1".into(),
        title: title.into(),
        target_date: date.into(),
        note: None,
        created_at: "2026-01-01T00:00:00+08:00".into(),
    }
}

#[test]
fn validate_items_accepts_valid_records() {
    assert!(validate_items(&[item("测试", "2030-01-01")]).is_ok());
    assert!(validate_items(&[]).is_ok());
}

#[test]
fn validate_items_rejects_blank_title_and_bad_date() {
    assert!(validate_items(&[item("  ", "2030-01-01")]).is_err());
    assert!(validate_items(&[item("测试", "not-a-date")]).is_err());
}

#[test]
fn envelope_parses_by_kind() {
    let items_file = r#"{"kind":"items","items":[{"id":"it-1","title":"a","target_date":"2030-01-01","note":null,"created_at":""}]}"#;
    let p: ImportEnvelope = serde_json::from_str(items_file).unwrap();
    assert_eq!(p.kind, KIND_ITEMS);
    assert_eq!(p.items.unwrap().len(), 1);

    let settings_file =
        r##"{"kind":"settings","settings":{"levels":[{"threshold_days":5,"color":"#ffffff"}]}}"##;
    let p: ImportEnvelope = serde_json::from_str(settings_file).unwrap();
    assert_eq!(p.kind, KIND_SETTINGS);
    assert!(p.items.is_none());
    let s = p.settings.unwrap();
    assert_eq!(s.levels[0].threshold_days, 5);
    assert_eq!(s.levels[0].color, "#ffffff");
}
