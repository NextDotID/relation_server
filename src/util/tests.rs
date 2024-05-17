use super::*;

#[test]
fn test_timestamp_to_naive_success() {
    let timestamp = timestamp_to_naive(1685522091, 0).unwrap();
    assert_eq!(
        timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
        "2023-05-31 08:34:51".to_string()
    );
}
