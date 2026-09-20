//! Worker 安全的 Unix 时间与 ISO-8601（Z）转换
//!
//! civil day 算法来自 Howard Hinnant 的 `chrono` 日期转换（公有领域思路，
//! 常见实现），用于在无 `time` crate 的 WASM 环境下处理日期。

pub fn now_ts() -> i64 {
    (worker::Date::now().as_millis() / 1000) as i64
}

/// Unix 秒 → `YYYY-MM-DDTHH:MM:SSZ`
pub fn format_unix_iso(ts: i64) -> String {
    let days = ts.div_euclid(86400);
    let secs = ts.rem_euclid(86400);
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    let (y, mo, d) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

/// 解析 `...Z` / 空格分隔的 ISO 时间为 Unix 秒；解析失败返回 0
pub fn parse_iso_pub(s: &str) -> i64 {
    let s = s.trim().replace('T', " ").replace('Z', "");
    let parts: Vec<&str> = s.split(' ').collect();
    if parts.len() < 2 {
        return 0;
    }
    let date: Vec<i64> = parts[0]
        .split('-')
        .filter_map(|x| x.parse().ok())
        .collect();
    let time: Vec<i64> = parts[1]
        .split(':')
        .filter_map(|x| x.parse().ok())
        .collect();
    if date.len() < 3 || time.len() < 2 {
        return 0;
    }
    let (h, mi, se) = (time[0], time[1], *time.get(2).unwrap_or(&0));
    days_from_civil(date[0], date[1] as u32, date[2] as u32) * 86400 + h * 3600 + mi * 60 + se
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
