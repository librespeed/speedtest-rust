use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{DateTime, Local};

pub fn convert_time_local (time : i64) -> String {
    let dt = DateTime::from_timestamp_millis(time).unwrap();
    let local : DateTime<Local> = DateTime::from(dt);
    local.format("%Y-%m-%d %H:%M:%S").to_string()
}

#[allow(dead_code)]
pub fn convert_time_utc (time : i64) -> String {
    let dt = DateTime::from_timestamp_millis(time).unwrap();
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn get_current_millis () -> i64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let in_ms = since_the_epoch.as_secs() * 1000 +
        since_the_epoch.subsec_nanos() as u64 / 1_000_000;
    in_ms as i64
}