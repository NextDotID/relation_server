use chrono::NaiveDateTime;

/// Returns current UNIX timestamp (unit: second).
pub fn timestamp() -> i64 {
    naive_now().timestamp()
}

/// Work as `NaiveDateTime::now()`
pub fn naive_now() -> NaiveDateTime {
    chrono::Utc::now().naive_utc()
}

/// Convert timestamp into NaiveDateTime struct.
pub fn timestamp_to_naive(ts: i64) -> NaiveDateTime {
    NaiveDateTime::from_timestamp(ts, 0)
}
