use super::{validate_items, ImportPayload};
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
fn import_payload_parses_without_settings() {
    let p: ImportPayload =
        serde_json::from_str(r#"{"items":[{"id":"it-1","title":"a","target_date":"2030-01-01","note":null,"created_at":""}]}"#)
            .unwrap();
    assert_eq!(p.items.len(), 1);
    assert!(p.settings.is_none());
}
