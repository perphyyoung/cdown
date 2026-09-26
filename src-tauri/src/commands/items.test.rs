use super::{new_id, validate_title_and_date};

#[test]
fn rejects_bad_date_format() {
    // chrono 的 %m/%d 解析对缺前导零宽松（"2030-1-1" 可过，type="date" 也不会产生这种值），
    // 只挡真正非法的日期
    assert!(validate_title_and_date("测试", "2030-1-1").is_ok());
    assert!(validate_title_and_date("测试", "2030-13-01").is_err());
    assert!(validate_title_and_date("测试", "not-a-date").is_err());
    assert!(validate_title_and_date("测试", "2030-01-01").is_ok());
}

#[test]
fn rejects_blank_title() {
    assert!(validate_title_and_date("   ", "2030-01-01").is_err());
}

#[test]
fn new_id_is_unique_in_process() {
    let a = new_id();
    let b = new_id();
    assert_ne!(a, b);
}
