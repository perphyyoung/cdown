//! hotkey.rs 的单元测试：accelerator 解析与规范化规则、热键切换判定

use super::{canonicalize, is_in_front, HotkeyHeld};

fn norm(raw: &str) -> Option<String> {
    canonicalize(Some(raw)).unwrap()
}

#[test]
fn keeps_canonical_form() {
    assert_eq!(norm("Ctrl+Alt+C").as_deref(), Some("Ctrl+Alt+C"));
}

#[test]
fn normalizes_case_aliases_and_key_prefix() {
    // 大小写不敏感、CONTROL 别名、主键带 Code 前缀、修饰键顺序固定 Ctrl/Alt/Shift/Super
    assert_eq!(norm("control+alt+KeyC").as_deref(), Some("Ctrl+Alt+C"));
    assert_eq!(norm("shift+ctrl+Digit1").as_deref(), Some("Ctrl+Shift+1"));
    assert_eq!(norm(" Alt + Ctrl + KeyK ").as_deref(), Some("Ctrl+Alt+K"));
    // 非字母数字键保留 Code 名
    assert_eq!(norm("Ctrl+Alt+F8").as_deref(), Some("Ctrl+Alt+F8"));
    assert_eq!(norm("Ctrl+Alt+Space").as_deref(), Some("Ctrl+Alt+Space"));
}

#[test]
fn empty_means_disabled() {
    assert_eq!(canonicalize(Some("")).unwrap(), None);
    assert_eq!(canonicalize(Some("   ")).unwrap(), None);
    assert_eq!(canonicalize(None).unwrap(), None);
}

#[test]
fn rejects_without_modifier() {
    // 裸按键会被全局抢占（普通打字都会被截走），一律拒绝
    assert!(canonicalize(Some("F8")).is_err());
    assert!(canonicalize(Some("C")).is_err());
}

#[test]
fn rejects_unparsable() {
    assert!(canonicalize(Some("Ctrl+Alt+没这个键")).is_err());
    assert!(canonicalize(Some("Ctrl+C+Alt")).is_err()); // 主键必须最后
    assert!(canonicalize(Some("Ctrl+")).is_err());
}

#[test]
fn front_window_only_when_visible_and_focused() {
    assert!(is_in_front(true, false, true)); // 正显示在前台 → 按热键收回托盘
    assert!(!is_in_front(false, false, false)); // 已藏在托盘 → 唤起
    assert!(!is_in_front(true, true, false)); // 最小化 → 唤起（unminimize）
    assert!(!is_in_front(true, false, false)); // 可见但被别的程序压住 → 唤起
}

#[test]
fn held_flag_only_fires_on_fresh_press() {
    let held = HotkeyHeld::default();
    assert!(held.press()); // 松手状态下的按下 = 一次有效切换
    assert!(!held.press()); // 长按的自动重复 → 忽略，避免反复 toggle
    held.release();
    assert!(held.press()); // 松手后再按
}
