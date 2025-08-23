//! 本项目的时间，都是服务器本机的时间

use chrono::{Duration, Local, NaiveDateTime, TimeZone};

use crate::common::WRITE_OFF_QRCODE_EXPIRES_SEC;

#[allow(unused)]
pub enum NowTimeType {
    Date,
    Time,
    DateTime,
}

/// 获取当前时间，，
/// ```
/// get_now_time(NowTimeType::DateTime)
/// // 2023-01-03 10:21:39
/// ```
pub fn get_now_time(t: NowTimeType) -> String {
    match t {
        NowTimeType::DateTime => {
            let fmt = "%Y-%m-%d %H:%M:%S";
            let parse = Local::now().format(fmt);
            return parse.to_string();
        }
        NowTimeType::Date => {
            let fmt = "%Y-%m-%d";
            let parse = Local::now().format(fmt);
            return parse.to_string();
        }
        NowTimeType::Time => {
            let fmt = "%H:%M:%S";
            let parse = Local::now().format(fmt);
            return parse.to_string();
        }
    }
}

/// 将时间戳转为日期 2023-01-03 10:21:39
/// 将时间戳转为日期
/// ```
/// timestamp_to_date(1672734099)
/// // 2023-01-03 10:21:39
/// ```
pub fn _timestamp_to_date(timestamp: i64) -> String {
    let fmt = "%Y-%m-%d %H:%M:%S";
    let date = Local.timestamp_opt(timestamp, 0).unwrap();
    date.format(fmt).to_string()
}

/// 对当前时间 进行加天数操作
/// ```
/// add_days("2023-01-03 10:43:49", 1)
/// 2023-01-04 10:43:49
/// ```
#[allow(unused)]
pub fn add_days(time: String, d: i64) -> String {
    let fmt = "%Y-%m-%d %H:%M:%S";
    let parse = NaiveDateTime::parse_from_str(time.as_str(), fmt).unwrap();
    let new_time = parse + Duration::days(d);
    return new_time.format(fmt).to_string();
}

/// 获取当前时间戳
pub fn _gen_now_timestamp() -> u64 {
    chrono::Local::now().timestamp() as u64
}
/// 生成当前时间起的过期时间戳
pub fn gen_now_expire_time() -> u64 {
    (chrono::Local::now().timestamp() + WRITE_OFF_QRCODE_EXPIRES_SEC) as u64
}
/// 验证 时间戳，与当前比，过没过期
pub fn is_expired(timestamp: u64) -> bool {
    let now = chrono::Local::now().timestamp() as u64;
    timestamp < now
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_days() {
        let original_time = "2023-01-03 10:43:49".to_string();
        let days_to_add = 1;
        let expected_result = "2023-01-04 10:43:49";

        let result = add_days(original_time, days_to_add);
        println!("Result of add_days: {}", result);

        assert_eq!(result, expected_result);
    }
}
